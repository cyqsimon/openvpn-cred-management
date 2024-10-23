use std::{
    any::type_name,
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{bail, eyre, Context, OptionExt};
use directories::ProjectDirs;
use documented::{Documented, DocumentedFields};
use itertools::Itertools;
use log::warn;
use serde::{Deserialize, Serialize};
use toml_edit::{ArrayOfTables, Decor, DocumentMut, RawString, Table};

use crate::types::CustomScriptsMap;

fn project_dirs() -> color_eyre::Result<ProjectDirs> {
    ProjectDirs::from("net", "scheimong", "openvpn-cred-management")
        .ok_or_eyre("Cannot determine your home directory")
}

pub fn default_config_path() -> color_eyre::Result<PathBuf> {
    let path = project_dirs()?.config_dir().join("config.toml");
    Ok(path)
}

/// A type-enforced relative owned path.
#[derive(Clone, Debug, derive_more::Deref, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PathBuf")]
pub struct RelativePathBuf(PathBuf);
impl TryFrom<PathBuf> for RelativePathBuf {
    type Error = color_eyre::Report;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        path.is_relative()
            .then_some(Self(path.clone()))
            .ok_or_else(|| eyre!("{path:?} is not relative"))
    }
}
impl TryFrom<&str> for RelativePathBuf {
    type Error = <Self as TryFrom<PathBuf>>::Error;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        PathBuf::from(path).try_into()
    }
}
impl AsRef<Path> for RelativePathBuf {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Options related to the `package-for` subcommand.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Documented, DocumentedFields)]
#[serde(rename_all = "kebab-case")]
#[documented_fields(rename_all = "kebab-case")]
pub struct Packaging {
    /// The skeleton directory that contains files to be included in all packages,
    /// relative to the location of this config file (if relative).
    ///
    /// Any contained symlinks will be followed.
    pub skel_dir: PathBuf,

    /// Scripts to be run on the skeleton directory before being used.
    ///
    /// These scripts are run on a temporary copy of the skeleton directory;
    /// the actual skeleton directory remains unchanged.
    pub skel_map_scripts: Vec<String>,

    /// The subpath within the skeleton directory to write the user's certificate.
    pub cert_subpath: RelativePathBuf,

    /// The subpath within the skeleton directory to write the user's key.
    pub key_subpath: RelativePathBuf,
}

/// Define a single profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Documented, DocumentedFields)]
#[serde(rename_all = "kebab-case")]
#[documented_fields(rename_all = "kebab-case")]
pub struct Profile {
    /// The identifier of the profile.
    pub name: String,

    /// The EasyRSA PKI directory.
    pub easy_rsa_pki_dir: PathBuf,

    /// Packaging settings.
    pub packaging: Option<Packaging>,

    /// Additional scripts to be run after running an action,
    /// defined separately for each type of action.
    ///
    /// These scripts are run in the current working directory.
    pub post_action_scripts: Option<CustomScriptsMap>,
}

/// The whole configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Documented, DocumentedFields)]
#[serde(try_from = "ConfigValidator", rename_all = "kebab-case")]
#[documented_fields(rename_all = "kebab-case")]
pub struct Config {
    /// The path to the EasyRSA executable.
    pub easy_rsa_path: PathBuf,

    /// The default profile to operate on.
    pub default_profile: Option<String>,

    /// The list of known profiles.
    #[serde(rename = "profile")]
    #[documented_fields(rename = "profile")]
    pub profiles: Vec<Profile>,
}
impl Config {
    /// Return an example config.
    pub fn example() -> Self {
        // autodetect which one is available
        let easy_rsa_path = [
            "/usr/share/easy-rsa/3/easyrsa", // Fedora
            "/usr/share/easy-rsa/easyrsa",   // Alpine, Debian
            "/usr/bin/easyrsa",              // Arch
        ]
        .into_iter()
        .map(Path::new)
        .find_or_first(|p| p.is_file())
        .unwrap() // first element always exists
        .to_owned();

        let packaging = Packaging {
            skel_dir: "skel/example/".into(),
            skel_map_scripts: vec![
                r#"echo "You can apply custom transforms to your skeleton directory""#.into(),
                r#"echo "before they are used to create user packages""#.into(),
            ],
            cert_subpath: "creds/client.crt".try_into().unwrap(),
            key_subpath: "creds/client.key".try_into().unwrap(),
        };
        let profile = Profile {
            name: "example".into(),
            easy_rsa_pki_dir: "/etc/openvpn/server/example.auth.d/".into(),
            packaging: Some(packaging),
            post_action_scripts: Some(CustomScriptsMap::example()),
        };

        Self {
            easy_rsa_path,
            default_profile: Some("example".into()),
            profiles: vec![profile],
        }
    }

    /// Create an annotated TOML document by inserting documentation.
    pub fn as_annotated_toml(&self) -> color_eyre::Result<DocumentMut> {
        let mut toml = toml_edit::ser::to_string_pretty(self)?.parse::<DocumentMut>()?;

        // annotate `Config`
        annotate_toml_table::<Config>(toml.as_table_mut(), true)
            .wrap_err("Failed to annotate `Config`")?;

        // annotate `Profile`
        let Some(profiles) = toml.get_mut("profile") else {
            return Ok(toml); // could be no profiles
        };
        let Some(profiles) = profiles.as_array_of_tables_mut() else {
            unreachable!("`profile` is not an array of tables");
        };
        annotate_toml_array_of_tables::<Profile>(profiles)
            .wrap_err("Failed to annotate `Profile`")?;

        // annotate `Packaging`
        for (i, profile) in profiles.iter_mut().enumerate() {
            let Some(packaging) = profile.get_mut("packaging") else {
                continue; // could be no packaging section
            };
            let Some(packaging) = packaging.as_table_mut() else {
                unreachable!("`packaging` is not a table");
            };
            annotate_toml_table::<Packaging>(packaging, false)
                .wrap_err_with(|| format!("Failed to annotate `Packaging` #{i}"))?;
        }

        Ok(toml)
    }

    /// Load the config from the specified path.
    pub fn load_from(config_path: impl AsRef<Path>) -> color_eyre::Result<Config> {
        let config_path = config_path.as_ref();

        let config_str = fs::read_to_string(config_path)
            .wrap_err_with(|| format!("Cannot read config file {config_path:?}"))?;

        let config = toml_edit::de::from_str(&config_str)
            .wrap_err_with(|| format!("Deserialising config file {config_path:?} failed"))?;

        Ok(config)
    }

    /// Get the profile with the given name.
    pub fn get_profile(&self, name: Option<impl AsRef<str>>) -> color_eyre::Result<&Profile> {
        let name = name
            .as_ref()
            .map(AsRef::as_ref)
            .or_else(|| self.default_profile.as_deref())
            .ok_or_eyre("No profile specified")?;
        self.profiles
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| eyre!(r#"Cannot find a profile named "{name}""#))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ConfigValidator {
    easy_rsa_path: PathBuf,
    default_profile: Option<String>,
    #[serde(rename = "profile")]
    profiles: Vec<Profile>,
}
impl TryFrom<ConfigValidator> for Config {
    type Error = color_eyre::Report;

    fn try_from(config: ConfigValidator) -> Result<Self, Self::Error> {
        let ConfigValidator { easy_rsa_path, default_profile, profiles } = config;

        // `default_profile` has to reference an existing profile
        if let Some(ref name) = default_profile {
            if profiles.iter().find(|p| &p.name == name).is_none() {
                bail!(
                    r#"The specified default profile "{name}" does not reference a known profile"#
                )
            }
        }

        Ok(Self { easy_rsa_path, default_profile, profiles })
    }
}

/// Insert annotations as comments into the serialised TOML representation of a
/// type using its doc comments.
///
/// Note that this function is not recursive. We do not descend into sub-tables
/// and sub-arrays-of-tables and annotate their fields; we only annotate the
/// sub-table or sub-arrays-of-tables themselves with the doc comments of their
/// corresponding fields on this type.
fn annotate_toml_table<T>(table: &mut Table, is_root: bool) -> color_eyre::Result<()>
where
    T: Documented + DocumentedFields,
{
    use toml_edit::Item as I;

    fn append_docs_as_toml_comments(decor: &mut Decor, docs: &str) {
        let old_prefix = decor.prefix().and_then(RawString::as_str);
        let last_line = old_prefix.and_then(|prefix| prefix.lines().last());

        let comments = docs
            .lines()
            .map(|l| if l.is_empty() { "#\n".into() } else { format!("# {l}\n") })
            .collect();

        let new_prefix = match (old_prefix, last_line) {
            // no prior comments
            (None | Some(""), None) => comments,
            // no prior comments, but somehow there are lines
            (None, Some(_)) => unreachable!(),
            // prior comments is contentful, but there are no lines
            (Some(_), None) => unreachable!(),
            // last line of prior comments is empty
            (Some(prefix), Some("")) => format!("{prefix}{comments}"),
            // last line of prior comments is contentful
            (Some(prefix), Some(_)) => format!("{prefix}#\n{comments}"),
        };
        decor.set_prefix(new_prefix);
    }

    // docs on this container
    if !is_root {
        append_docs_as_toml_comments(table.decor_mut(), T::DOCS);
    }

    // docs on fields
    for (mut key, value) in table.iter_mut() {
        // extract docs
        let field_name = key.get();
        let Ok(docs) = T::get_field_docs(&field_name) else {
            // ignore fields not known to `T`
            let ty = type_name::<T>();
            warn!("Encountered an unknown field `{field_name}` while annotating `{ty}`");
            continue;
        };

        // add comments
        match value {
            I::None => bail!("Encountered a `None` key unexpectedly"),
            I::Value(_) => append_docs_as_toml_comments(key.leaf_decor_mut(), docs),
            I::Table(sub_table) => append_docs_as_toml_comments(sub_table.decor_mut(), docs),
            I::ArrayOfTables(array) => {
                let first_table = array
                    .iter_mut()
                    .next()
                    .expect("Array of table should not be empty");
                append_docs_as_toml_comments(first_table.decor_mut(), docs);
            }
        }
    }

    Ok(())
}

/// Same as [`annotate_toml_table`], but annotate every table in the array.
fn annotate_toml_array_of_tables<T>(array: &mut ArrayOfTables) -> color_eyre::Result<()>
where
    T: Documented + DocumentedFields,
{
    for (i, table) in array.iter_mut().enumerate() {
        annotate_toml_table::<T>(table, false)
            .wrap_err_with(|| format!("Failed to annotate table {i}"))?;
    }
    Ok(())
}

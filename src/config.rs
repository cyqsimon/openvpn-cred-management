use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{bail, eyre, OptionExt};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Packaging {
    /// The skeleton directory that contains files to be included in all packages.
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
    /// The identifier of the profile.
    pub name: String,

    /// The EasyRSA PKI directory.
    pub easy_rsa_pki_dir: PathBuf,

    /// Packaging settings.
    pub packaging: Option<Packaging>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConfigValidator", rename_all = "kebab-case")]
pub struct Config {
    /// The path to the EasyRSA executable.
    pub easy_rsa_path: PathBuf,

    /// The default profile to operate on.
    pub default_profile: Option<String>,

    /// The list of known profiles.
    pub profiles: Vec<Profile>,
}
impl Config {
    /// Return an example config.
    fn example() -> Self {
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
        };
        Self {
            easy_rsa_path: "/usr/share/easy-rsa/3/easyrsa".into(),
            default_profile: Some("example".into()),
            profiles: vec![profile],
        }
    }

    /// Read the config from the specified path, or create a new example config
    /// at this path if it does not exist.
    pub fn read_or_init(config_path: impl AsRef<Path>) -> color_eyre::Result<Config> {
        let config_path = config_path.as_ref();

        let config = if config_path.is_file() {
            let config_str = fs::read_to_string(config_path)?;
            toml::from_str(&config_str)?
        } else {
            // create parent dir
            let parent = config_path
                .parent()
                .ok_or_eyre(format!("Cannot get parent directory of {config_path:?}"))?;
            fs::create_dir_all(parent)?;

            // create config
            let config = Config::example();
            fs::write(config_path, toml::to_string_pretty(&config)?)?;

            config
        };
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

use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{eyre, OptionExt};
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
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The path to the EasyRSA executable.
    pub easy_rsa_path: PathBuf,

    /// The list of known profiles.
    pub profiles: Vec<Profile>,
}
impl Config {
    /// Return an example config.
    fn example() -> Self {
        let packaging = Packaging {
            skel_dir: "skel/example/".into(),
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
}

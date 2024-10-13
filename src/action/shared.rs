use std::{
    borrow::Cow,
    collections::BTreeSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::eyre;
use log::warn;

use crate::{config::Profile, types::Username};

pub fn get_users(
    config_dir: impl AsRef<Path>,
    profile: &Profile,
) -> color_eyre::Result<Vec<Username>> {
    fn list_names(dir: impl AsRef<Path>) -> color_eyre::Result<BTreeSet<OsString>> {
        let dir = dir.as_ref();
        let names = fs::read_dir(dir)?
            .filter_map(|de| {
                de.inspect_err(|e| {
                    warn!("Failed to read a file in {dir:?}; the user list may be incomplete");
                    warn!("{e}");
                })
                .ok()
            })
            .filter_map(|de| {
                let path = de.path();
                if !path.is_file() {
                    warn!("{path:?} is not a regular file; ignoring");
                    return None;
                }
                match path.file_stem() {
                    Some(stem) => Some(stem.to_owned()),
                    None => {
                        warn!("{path:?} does not have a file stem; ignoring");
                        None
                    }
                }
            })
            .collect();
        Ok(names)
    }

    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.as_ref().join(&profile.easy_rsa_pki_dir);

    let cert_names = list_names(pki_dir.join("issued"))?;
    let key_names = list_names(pki_dir.join("private"))?;

    cert_names
        .difference(&key_names)
        .for_each(|n| warn!("User {n:?} seems to have a certificate but no key"));
    key_names
        .difference(&cert_names)
        .for_each(|n| warn!("User {n:?} seems to have a key but no certificate"));

    let output = cert_names
        .union(&key_names)
        .filter_map(|n| {
            let s = n.to_string_lossy();
            if let Cow::Owned(_) = s {
                warn!("User {n:?} seems to have a non-UTF8 name");
            }
            s.parse::<Username>()
                .inspect_err(|err| warn!("The username {s:?} failed parsing; ignoring: {err:?}"))
                .ok()
        })
        .collect();

    Ok(output)
}

pub fn get_cert_path(
    config_dir: impl AsRef<Path>,
    profile: &Profile,
    username: &Username,
) -> color_eyre::Result<PathBuf> {
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.as_ref().join(&profile.easy_rsa_pki_dir);

    let path = pki_dir.join("issued").join(format!("{username}.crt"));
    path.is_file()
        .then_some(path)
        .ok_or_else(|| eyre!(r#"Cannot find a certificate for user "{username}""#))
}

pub fn get_key_path(
    config_dir: impl AsRef<Path>,
    profile: &Profile,
    username: &Username,
) -> color_eyre::Result<PathBuf> {
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.as_ref().join(&profile.easy_rsa_pki_dir);

    let path = pki_dir.join("private").join(format!("{username}.key"));
    path.is_file()
        .then_some(path)
        .ok_or_else(|| eyre!(r#"Cannot find a key for user "{username}""#))
}

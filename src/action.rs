mod shared;

use std::path::Path;

use color_eyre::eyre::bail;
use itertools::Itertools;
use xshell::{cmd, Shell};

use crate::{
    action::shared::get_users,
    config::{Config, Profile},
    types::Username,
};

pub fn list_users(config_dir: impl AsRef<Path>, profile: &Profile) -> color_eyre::Result<()> {
    let output = get_users(config_dir, profile)?.into_iter().join("\n");
    println!("{output}");
    Ok(())
}

pub fn new_user(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
    username: &Username,
    days: Option<usize>,
) -> color_eyre::Result<()> {
    let config_dir = config_dir.as_ref();

    if get_users(config_dir, profile)?.contains(username) {
        bail!("{username} already exists in profile {p}", p = profile.name);
    }

    let easy_rsa = &config.easy_rsa_path;
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.join(&profile.easy_rsa_pki_dir);
    let days_arg = days.map(|d| format!("--days={d}"));

    let sh = Shell::new()?;
    cmd!(
        sh,
        "{easy_rsa} --batch --pki-dir={pki_dir} --no-pass {days_arg...} build-client-full {username}"
    )
    .run()?;

    Ok(())
}

pub fn remove_user(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
    username: &Username,
    update_crl: bool,
) -> color_eyre::Result<()> {
    let config_dir = config_dir.as_ref();

    if !get_users(config_dir, profile)?.contains(username) {
        bail!(
            "{username} does not exists in profile {p}",
            p = profile.name
        );
    }

    let easy_rsa = &config.easy_rsa_path;
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.join(&profile.easy_rsa_pki_dir);

    let sh = Shell::new()?;
    cmd!(
        sh,
        "{easy_rsa} --batch --pki-dir={pki_dir} revoke {username}"
    )
    .run()?;
    if update_crl {
        cmd!(sh, "{easy_rsa} --batch --pki-dir={pki_dir} gen-crl").run()?;
    }

    Ok(())
}

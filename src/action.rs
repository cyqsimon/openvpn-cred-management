mod shared;

use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use color_eyre::eyre::{bail, eyre, Context};
use fs_more::directory::{
    copy_directory, DirectoryCopyDepthLimit, DirectoryCopyOptions, SymlinkBehaviour,
};
use itertools::Itertools;
use log::info;
use temp_dir::TempDir;
use xshell::{cmd, Shell};
use zip::ZipWriter;
use zip_extensions::ZipWriterExtensions;

use crate::{
    action::shared::{get_cert_path, get_key_path, get_users},
    config::{Config, Profile},
    types::Username,
};

pub fn init_config(config_path: impl AsRef<Path>, allow_overwrite: bool) -> color_eyre::Result<()> {
    let config_path = config_path.as_ref();

    // create parent dir
    let parent = config_path
        .parent()
        .ok_or_else(|| eyre!("Cannot get parent directory of {config_path:?}"))?;
    fs::create_dir_all(parent).wrap_err_with(|| format!("Cannot create directory {parent:?}"))?;
    info!("Created directory {parent:?}");

    // create config
    let config = Config::example()
        .as_annotated_toml()
        .wrap_err("Cannot annotate the default config")?;

    // write
    let mut config_file = if allow_overwrite {
        File::create(config_path)
    } else {
        File::create_new(config_path)
    }
    .wrap_err_with(|| format!("Cannot create new config file {config_path:?}"))?;
    config_file
        .write_all(config.to_string().as_bytes())
        .wrap_err_with(|| format!("Failed to write config file to {config_path:?}"))?;
    info!("Created example config file at {config_path:?}");

    Ok(())
}

pub fn list_users(config_dir: impl AsRef<Path>, profile: &Profile) -> color_eyre::Result<()> {
    let profile_name = &profile.name;

    let output = get_users(config_dir, profile)
        .wrap_err_with(|| format!(r#"Cannot get users of "{profile_name}" profile"#))?
        .into_iter()
        .join("\n");
    println!("{output}");
    Ok(())
}

pub fn new_user(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
    usernames: &[Username],
    days: Option<usize>,
) -> color_eyre::Result<()> {
    let config_dir = config_dir.as_ref();
    let profile_name = &profile.name;

    // sanity check
    let known_users = get_users(config_dir, profile)
        .wrap_err_with(|| format!(r#"Cannot get users of "{profile_name}" profile"#))?;
    for username in usernames {
        if known_users.contains(username) {
            bail!(r#"User "{username}" already exists in profile "{profile_name}""#);
        }
    }

    let easy_rsa = &config.easy_rsa_path;
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.join(&profile.easy_rsa_pki_dir);
    let days_arg = days.map(|d| format!("--days={d}"));
    let days_arg = days_arg.as_ref(); // otherwise use of moved value

    let sh = Shell::new().wrap_err("Failed to create subshell")?;
    for username in usernames {
        cmd!(
            sh,
            "{easy_rsa} --batch --pki-dir={pki_dir} --no-pass {days_arg...} build-client-full {username}"
        )
        .run().wrap_err("User creation command failed to execute")?;
    }

    Ok(())
}

pub fn remove_user(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
    usernames: &[Username],
    update_crl: bool,
) -> color_eyre::Result<()> {
    let config_dir = config_dir.as_ref();
    let profile_name = &profile.name;

    let known_users = get_users(config_dir, profile)
        .wrap_err_with(|| format!(r#"Cannot get users of "{profile_name}" profile"#))?;
    for username in usernames {
        if !known_users.contains(username) {
            bail!(r#"User "{username}" does not exists in profile "{profile_name}""#);
        }
    }

    let easy_rsa = &config.easy_rsa_path;
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.join(&profile.easy_rsa_pki_dir);

    let sh = Shell::new().wrap_err("Failed to create subshell")?;
    for username in usernames {
        cmd!(
            sh,
            "{easy_rsa} --batch --pki-dir={pki_dir} revoke {username}"
        )
        .run()
        .wrap_err("User deletion command failed to execute")?;
    }

    if update_crl {
        cmd!(sh, "{easy_rsa} --batch --pki-dir={pki_dir} gen-crl")
            .run()
            .wrap_err("CRL update command failed to execute")?;
    }

    Ok(())
}

pub fn package(
    config_dir: impl AsRef<Path>,
    profile: &Profile,
    usernames: &[Username],
    add_prefix: bool,
    output_dir: impl AsRef<Path>,
) -> color_eyre::Result<()> {
    let config_dir = config_dir.as_ref();
    let profile_name = &profile.name;
    let output_dir = output_dir.as_ref();

    // sanity checks
    let Some(ref packaging) = profile.packaging else {
        bail!(r#"Profile "{profile_name}" does not contain a "packaging" section"#);
    };

    let known_users = get_users(config_dir, profile)
        .wrap_err_with(|| format!(r#"Cannot get users of "{profile_name}" profile"#))?;
    for username in usernames {
        if !known_users.contains(username) {
            bail!(r#"User "{username}" does not exists in profile "{profile_name}""#);
        }
    }

    // allow `skel_dir` to be relative to the config file
    let skel_dir = config_dir.join(&packaging.skel_dir);

    // copy skeleton directory
    let temp_dir = TempDir::with_prefix("openvpn-cred-management-")
        .wrap_err("Cannot create temporary working directory")?;
    let mapped_skel_dir = temp_dir.path().join("mapped-skel");
    let copy_options = DirectoryCopyOptions {
        copy_depth_limit: DirectoryCopyDepthLimit::Limited { maximum_depth: 64 },
        symlink_behaviour: SymlinkBehaviour::Follow,
        ..Default::default()
    };
    copy_directory(&skel_dir, &mapped_skel_dir, copy_options).wrap_err_with(|| {
        format!("Failed to copy skeleton directory {skel_dir:?} to {mapped_skel_dir:?}")
    })?;

    // apply transforms
    let sh = Shell::new().wrap_err("Failed to create subshell")?;
    sh.change_dir(&mapped_skel_dir);
    for script in &packaging.skel_map_scripts {
        cmd!(sh, "bash -c {script}")
            .run()
            .wrap_err("A skeleton transform script failed to execute")?;
    }
    drop(sh);

    // create parent dir for individual packages
    let pkg_parent_dir = temp_dir.path().join("pkgs");
    fs::create_dir_all(&pkg_parent_dir).wrap_err_with(|| {
        format!("Failed to create packages' parent directory {pkg_parent_dir:?}")
    })?;

    // package for each user
    for username in usernames {
        // copy skeleton directory
        let pkg_dir = pkg_parent_dir.join(username);
        copy_directory(&mapped_skel_dir, &pkg_dir, Default::default()).wrap_err_with(|| {
            format!(
                "Failed to copy transformed skeleton directory {mapped_skel_dir:?} to {pkg_dir:?}"
            )
        })?;

        // create subdirectories for certificate and key
        for subpath in [&packaging.cert_subpath, &packaging.key_subpath] {
            match subpath.parent() {
                Some(parent) if parent != Path::new("") => {
                    let full_dir_path = pkg_dir.join(parent);
                    fs::create_dir_all(&full_dir_path).wrap_err_with(|| {
                        format!(
                            "Failed to create parent path {full_dir_path:?} for certificate or key"
                        )
                    })?
                }
                Some(_) | None => (), // no intermediate directories to create
            }
        }

        // copy certificate
        let cert_source_path =
            get_cert_path(config_dir, profile, username).wrap_err_with(|| {
                format!(r#"Failed to get certificate path for user "{username}" in profile "{profile_name}""#)
            })?;
        let cert_target_path = pkg_dir.join(&packaging.cert_subpath);
        fs::copy(&cert_source_path, &cert_target_path).wrap_err_with(|| {
            format!(r#"Failed to copy certificate {cert_source_path:?} to {cert_target_path:?}"#)
        })?;

        // copy key
        let key_source_path = get_key_path(config_dir, profile, username).wrap_err_with(|| {
            format!(r#"Failed to get key path for user "{username}" in profile "{profile_name}""#)
        })?;
        let key_target_path = pkg_dir.join(&packaging.key_subpath);
        fs::copy(&key_source_path, &key_target_path).wrap_err_with(|| {
            format!(r#"Failed to copy key {key_source_path:?} to {key_target_path:?}"#)
        })?;

        // write archive
        let archive_name = if add_prefix {
            format!("{profile_name}-{username}.zip")
        } else {
            format!("{username}.zip")
        };
        let zip_file = File::create_new(output_dir.join(&archive_name))
            .wrap_err_with(|| format!(r#"Failed to create "{archive_name}" for output"#))?;
        let zip_writer = ZipWriter::new(zip_file);
        zip_writer
            .create_from_directory(&pkg_dir)
            .wrap_err_with(|| format!(r#"Failed while writing into "{archive_name}""#))?;
    }

    Ok(())
}

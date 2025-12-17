use std::{
    borrow::Cow,
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, NaiveDate, Utc};
use color_eyre::eyre::{eyre, Context};
use log::{debug, trace, warn};
use regex::Regex;
use xshell::{cmd, Shell};

use crate::{
    config::{Config, Profile},
    types::Username,
};

/// Get the number of days before year 10000.
///
/// This is the maximum number of days allowable for the `--days` option,
/// before OpenSSL starts complaining.
pub fn get_max_days() -> i64 {
    const TARGET_DATE: DateTime<Utc> = NaiveDate::from_ymd_opt(9999, 12, 31)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    (TARGET_DATE - Utc::now()).num_days()
}

pub fn get_users(
    config_dir: impl AsRef<Path>,
    profile: &Profile,
) -> color_eyre::Result<Vec<Username>> {
    fn list_names(dir: impl AsRef<Path>) -> color_eyre::Result<BTreeSet<OsString>> {
        let dir = dir.as_ref();
        let names = fs::read_dir(dir)
            .wrap_err_with(|| format!("Failed to read {dir:?}"))?
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

    // list all certificates
    let cert_dir = pki_dir.join("issued");
    let cert_names = list_names(&cert_dir)
        .wrap_err_with(|| format!("Cannot read certificate directory {cert_dir:?}"))?;

    // list all keys
    let key_dir = pki_dir.join("private");
    let key_names = {
        let mut names = list_names(&key_dir)
            .wrap_err_with(|| format!("Cannot read key directory {key_dir:?}"))?;
        names.remove(OsStr::new("ca")); // filter out the CA's key
        names
    };

    // warn about difference
    cert_names
        .difference(&key_names)
        .for_each(|n| warn!("User {n:?} seems to have a certificate but no key"));
    key_names
        .difference(&cert_names)
        .for_each(|n| warn!("User {n:?} seems to have a key but no certificate"));

    // build output
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

pub fn get_expired_users(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
) -> color_eyre::Result<Vec<Username>> {
    let easy_rsa = &config.easy_rsa_path;
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.as_ref().join(&profile.easy_rsa_pki_dir);
    let days_arg = format!("--days={}", get_max_days());

    let sh = Shell::new().wrap_err("Failed to create subshell")?;
    let show_expire_output = cmd!(sh, "{easy_rsa} --pki-dir={pki_dir} {days_arg} show-expire")
        .read()
        .wrap_err("List expired command failed to execute")?;
    debug!("`easy-rsa show-expire` output: {show_expire_output}");

    // easy-rsa's output format of each line that describes a certificate
    static LINE_MATCHER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^V \| Serial: (?<serial>[\dA-F]+) \| (Expire(s|d): )?(?<date>[\d\-]+) (?<time>[\d:Z+\-]+) \| CN: (?<name>[^\s]+)$",
        )
        .unwrap()
    });
    let now = Utc::now();

    let expired = show_expire_output
        .lines()
        .filter_map(|line| {
            let Some(captures) = LINE_MATCHER.captures(line) else {
                trace!("`{line}` does not look like a certificate line");
                return None;
            };

            let name = {
                let raw = captures.name("name").unwrap().as_str(); // capture always exists
                raw.parse::<Username>().inspect_err(|err| {
                    warn!(r#"The username "{raw}" failed parsing; ignoring: {err:?}"#)
                })
            }
            .ok()?;

            let expiry = {
                let date = captures.name("date").unwrap().as_str(); // capture always exists
                let time = captures.name("time").unwrap().as_str(); // capture always exists
                DateTime::parse_from_rfc3339(&format!("{date}T{time}")).inspect_err(|_| {
                    warn!(
                        "easy-rsa reported expiry time of `{name}` \
                        in an unexpected format: `{date} {time}`"
                    )
                })
            }
            .ok()?;

            (now > expiry).then_some(name)
        })
        .collect();

    Ok(expired)
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

pub fn regenerate_crl(
    config_dir: impl AsRef<Path>,
    config: &Config,
    profile: &Profile,
    force: bool,
) -> color_eyre::Result<()> {
    let easy_rsa = &config.easy_rsa_path;
    let force_arg = force.then_some("--batch");
    // allow `easy_rsa_pki_dir` to be relative to the config file
    let pki_dir = config_dir.as_ref().join(&profile.easy_rsa_pki_dir);
    // an expired CRL causes all clients to be rejected
    // this CRL is self-managed anyways, so we set it to practically-unlimited
    let days_arg = format!("--days={}", get_max_days());

    let sh = Shell::new().wrap_err("Failed to create subshell")?;
    cmd!(
        sh,
        "{easy_rsa} {force_arg...} --pki-dir={pki_dir} {days_arg} gen-crl"
    )
    .run_interactive()
    .wrap_err("CRL regenerate command failed to execute")?;

    Ok(())
}

use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use color_eyre::eyre::OptionExt;
use regex::Regex;

#[derive(Clone, Debug, Parser)]
#[command(author, version)]
pub struct CliArgs {
    /// Path to the configuration file.
    ///
    /// Defaults to the OS-dependent project config directory for `net.scheimong/openvpn-cred-management`.
    /// See https://docs.rs/directories/5/directories/struct.ProjectDirs.html#method.config_dir.
    #[arg(short = 'c', long = "config", value_name = "PATH")]
    pub config_path: Option<PathBuf>,

    /// Manually select a profile to operate on.
    ///
    /// You can also specify a default profile in the config file.
    #[arg(short = 'p', long = "profile", value_name = "NAME")]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub action: Action,

    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Action {
    /// List all valid certificates.
    #[command(visible_aliases = ["ls"])]
    List,

    /// Generate a certificate for a new user.
    #[command(visible_aliases = ["new"])]
    NewUser {
        /// The username of the certificate to generate.
        #[arg(index = 1, value_name = "NAME")]
        username: Username,

        /// The number of days this certificate stays valid.
        #[arg(short = 'd', long = "days", value_name = "N")]
        days: Option<usize>,
    },

    /// Revoke the certificate for an existing user.
    #[command(visible_aliases = ["rm"])]
    RmUser {
        /// The username of the user to revoke.
        #[arg(index = 1, value_name = "NAME")]
        username: Username,

        /// Do not update crl.pem file.
        #[arg(long = "no-update-crl")]
        no_update_crl: bool,
    },

    /// Create redistributable packages for the specified users.
    #[command(visible_aliases = ["pkg", "package"])]
    PackageFor {
        /// The usernames of the users to package for.
        #[arg(index = 1, value_name = "NAME", required = true)]
        usernames: Vec<Username>,

        /// Add the profile name as a prefix to the package name.
        #[arg(short = 'p', long = "add-prefix")]
        add_prefix: bool,

        /// Output to a directory other than the current working directory.
        #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
        output_dir: Option<PathBuf>,
    },
}

/// A validated username.
#[derive(Clone, Debug, derive_more::Display)]
pub struct Username(String);
impl FromStr for Username {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const REGEX: &str = r"[\w\d\-_]+";
        static VALIDATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(REGEX).unwrap());
        VALIDATOR
            .is_match(s)
            .then(|| Self(s.to_owned()))
            .ok_or_eyre(r#"Username must match "{REGEX}""#)
    }
}

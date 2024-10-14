use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use serde::{Deserialize, Serialize};

use crate::types::Username;

#[derive(Clone, Debug, Parser)]
#[command(author, version)]
pub struct CliArgs {
    /// Path to the configuration file.
    ///
    /// Defaults to the OS-dependent project config directory for `net.scheimong/openvpn-cred-management`.
    /// See https://docs.rs/directories/5/directories/struct.ProjectDirs.html#method.config_dir.
    #[arg(short = 'c', long = "config", value_name = "PATH", global = true)]
    pub config_path: Option<PathBuf>,

    /// Manually select a profile to operate on.
    ///
    /// You can also specify a default profile in the config file.
    #[arg(short = 'p', long = "profile", value_name = "NAME", global = true)]
    pub profile: Option<String>,

    /// Do not run post-action scripts.
    #[arg(long = "no-post-action-scripts", visible_aliases = ["no-post-scripts"], global = true)]
    pub no_post_action_scripts: bool,

    #[command(subcommand)]
    pub action: Action,

    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[derive(Clone, Debug, Subcommand, strum::EnumDiscriminants)]
#[strum_discriminants(
    name(ActionType),
    derive(
        strum::Display,
        Hash,
        Ord,
        PartialOrd,
        strum::EnumIter,
        Serialize,
        Deserialize
    ),
    strum(serialize_all = "kebab-case"),
    serde(rename_all = "kebab-case")
)]
pub enum Action {
    /// Initialise a config file.
    ///
    /// If `config_path` is not specified, the default location is used.
    #[command(visible_aliases = ["init"])]
    InitConfig,

    /// List all valid certificates.
    #[command(visible_aliases = ["ls"])]
    List,

    /// Generate a certificate for a new user.
    #[command(visible_aliases = ["new"])]
    NewUser {
        /// The usernames of the certificates to generate.
        #[arg(index = 1, value_name = "NAME", required = true)]
        usernames: Vec<Username>,

        /// The number of days this certificate stays valid.
        #[arg(short = 'd', long = "days", value_name = "N")]
        days: Option<usize>,
    },

    /// Revoke the certificate for an existing user.
    #[command(visible_aliases = ["rm"])]
    RmUser {
        /// The usernames of the users to revoke.
        #[arg(index = 1, value_name = "NAME", required = true)]
        usernames: Vec<Username>,

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

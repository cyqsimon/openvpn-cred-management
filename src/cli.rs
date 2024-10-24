use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use clap_complete::Shell;
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
    #[arg(short = 'c', long = "config", value_name = "PATH", value_hint = ValueHint::FilePath, global = true)]
    pub config_path: Option<PathBuf>,

    /// Manually select a profile to operate on.
    ///
    /// You can also specify a default profile in the config file.
    #[arg(short = 'p', long = "profile", value_name = "NAME", global = true)]
    pub profile: Option<String>,

    /// Proceed with potentially destructive actions automatically without confirmation.
    #[arg(short = 'f', long = "force", global = true)]
    pub force: bool,

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
    /// Generate shell completion to stdout.
    Complete {
        /// Specify the shell to generate completion for.
        #[arg(index = 1, value_name = "KIND")]
        shell: Option<Shell>,
    },

    /// Initialise a config file.
    ///
    /// If `config_path` is not specified, the default location is used.
    #[command(visible_aliases = ["init"])]
    InitConfig,

    /// List all known profiles.
    #[command(visible_aliases = ["profiles"])]
    ListProfiles,

    /// List all certificates, with optional filtering.
    #[command(visible_aliases = ["ls"])]
    List {
        /// Only show expired certificates.
        #[arg(short = 'e', long = "expired")]
        only_expired: bool,
    },

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
        #[arg(long = "add-prefix", visible_aliases = ["pre"])]
        add_prefix: bool,

        /// Output to a directory other than the current working directory.
        #[arg(short = 'o', long = "output-dir", value_name = "DIR", value_hint = ValueHint::DirPath)]
        output_dir: Option<PathBuf>,

        /// Keep temporary intermediate artifacts instead of deleting them.
        /// Helpful for debugging.
        #[arg(long = "keep-temp")]
        keep_temp: bool,
    },
}

mod action;
mod cli;
mod config;
mod types;

use std::{env, path::Path};

use clap::Parser;
use color_eyre::eyre::bail;
use simplelog::{ColorChoice, TermLogger, TerminalMode};

use crate::{
    action::{list_users, new_user, package, remove_user},
    cli::{Action, CliArgs},
    config::{default_config_path, Config},
};

fn main() -> color_eyre::Result<()> {
    // install panic & error report handlers
    color_eyre::install()?;

    // parse CLI
    let CliArgs {
        config_path,
        profile,
        no_post_action_scripts,
        action,
        verbosity,
    } = CliArgs::parse();

    // init logging
    let logger_config = simplelog::ConfigBuilder::new().build();
    TermLogger::init(
        verbosity.log_level_filter(),
        logger_config,
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )?;

    // get config
    let config_path = match config_path {
        Some(p) => p,
        None => default_config_path()?,
    };
    let config = Config::read_or_init(&config_path)?;
    let config_dir = match config_path.parent() {
        Some(parent) if parent != Path::new("") => parent,
        Some(_) => Path::new("."), // current directory
        None => bail!("Cannot get the parent directory of {config_path:?}"),
    };

    // actions
    let profile = config.get_profile(profile)?;
    match action {
        Action::List => list_users(config_dir, profile)?,
        Action::NewUser { usernames, days } => {
            new_user(config_dir, &config, profile, &usernames, days)?
        }
        Action::RmUser { usernames, no_update_crl } => {
            remove_user(config_dir, &config, profile, &usernames, !no_update_crl)?
        }
        Action::PackageFor { usernames, add_prefix, output_dir } => {
            let output_dir = output_dir.unwrap_or(env::current_dir()?);
            package(config_dir, profile, &usernames, add_prefix, output_dir)?;
        }
    }

    Ok(())
}

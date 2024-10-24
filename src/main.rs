mod action;
mod cli;
mod config;
mod types;

use std::{env, path::Path};

use clap::Parser;
use color_eyre::eyre::{bail, Context};
use simplelog::{ColorChoice, TermLogger, TerminalMode};

use crate::{
    action::{init_config, list_profiles, list_users, new_user, package, remove_user},
    cli::{Action, ActionType, CliArgs},
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
    )
    .wrap_err("Failed to initialise logger")?;

    // get config path
    let config_path = match config_path {
        Some(p) => p,
        None => default_config_path()
            .wrap_err("No config path specified, and failed to get default config path")?,
    };
    let config_dir = match config_path.parent() {
        Some(parent) if parent != Path::new("") => parent,
        Some(_) => Path::new("."), // current directory
        None => bail!("Cannot get the parent directory of {config_path:?}"),
    };

    // handle config init
    if let Action::InitConfig { force } = &action {
        init_config(&config_path, *force)
            .wrap_err_with(|| format!("Failed to initialise config {config_path:?}"))?;
        return Ok(());
    }

    // load config
    let config = Config::load_from(&config_path)
        .wrap_err_with(|| format!("Failed to load config {config_path:?}"))?;

    // get profile
    let profile = config
        .get_profile_or_default(profile)
        .wrap_err("Cannot select a profile")?;
    let profile_name = &profile.name;

    // other actions
    match &action {
        Action::InitConfig { .. } => unreachable!(), // already handled
        Action::ListProfiles => {
            list_profiles(&config, profile).wrap_err("Failed to list known profiles")?
        }
        Action::List => list_users(config_dir, profile)
            .wrap_err_with(|| format!(r#"Failed to list users of profile "{profile_name}""#))?,
        Action::NewUser { usernames, days } => {
            new_user(config_dir, &config, profile, &usernames, *days).wrap_err_with(|| {
                format!(r#"Failed while adding users to profile "{profile_name}""#)
            })?
        }
        Action::RmUser { usernames, no_update_crl } => {
            remove_user(config_dir, &config, profile, &usernames, !no_update_crl).wrap_err_with(
                || format!(r#"Failed while removing users from profile "{profile_name}""#),
            )?
        }
        Action::PackageFor {
            usernames,
            add_prefix,
            output_dir,
            keep_temp,
        } => {
            let output_dir = match output_dir {
                Some(dir) => dir.to_owned(),
                None => env::current_dir().wrap_err(
                    "No output directory specified, and failed to get current working directory",
                )?,
            };
            package(
                config_dir,
                profile,
                &usernames,
                *add_prefix,
                output_dir,
                *keep_temp,
            )
            .wrap_err_with(|| {
                format!(r#"Failed while packaging users of profile "{profile_name}""#)
            })?;
        }
    }

    // post-action scripts
    if !no_post_action_scripts {
        let action_type = ActionType::from(&action);
        if let Some(scripts) = &profile.post_action_scripts {
            scripts
                .run_for(action_type)
                .wrap_err("Failed while running post-action scripts")?;
        };
    }

    Ok(())
}

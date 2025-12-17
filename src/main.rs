mod action;
mod cli;
mod config;
mod types;

use std::{env, io, path::Path};

use clap::{CommandFactory, Parser};
use color_eyre::eyre::{bail, Context};
use simplelog::{ColorChoice, TermLogger, TerminalMode};

use crate::{
    action::{
        info_user, init_config, list_expired, list_profiles, list_users, new_user, package,
        remove_user,
    },
    cli::{Action, CliArgs, GenAction, ProfileAction, UserAction},
    config::{default_config_path, Config, Profile},
};

fn main() -> color_eyre::Result<()> {
    // install panic & error report handlers
    color_eyre::install()?;

    // parse CLI
    let CliArgs {
        config_path,
        profile,
        force,
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

    // handle completion generation
    if let Action::Gen { action: GenAction::Completion { shell } } = &action {
        let Some(shell) = shell.or_else(clap_complete::Shell::from_env) else {
            bail!("Failed to determine your shell; please specify one manually.")
        };
        let mut cmd = CliArgs::command();
        clap_complete::generate(shell, &mut cmd, "ocm", &mut io::stdout());
        return Ok(());
    }

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
    if let Action::Gen { action: GenAction::Config } = &action {
        init_config(&config_path, force)
            .wrap_err_with(|| format!("Failed to initialise config {config_path:?}"))?;
        return Ok(());
    }

    // load config
    let config = Config::load_from(&config_path)
        .wrap_err_with(|| format!("Failed to load config {config_path:?}"))?;

    // get profile
    let profile = config
        .get_profile_or_default(profile.as_ref())
        .wrap_err("Cannot select a profile")?;
    let profile_name = &profile.name;

    // other actions
    match &action {
        Action::Gen { .. } => unreachable!(), // already handled
        Action::Profile { action } => match action {
            ProfileAction::List => list_profiles(&config, profile),
        },
        Action::User { action } => match action {
            UserAction::List { only_expired } => {
                if *only_expired {
                    list_expired(config_dir, &config, profile).wrap_err_with(|| {
                        format!(r#"Failed to list expired users of profile "{profile_name}""#)
                    })?
                } else {
                    list_users(config_dir, profile).wrap_err_with(|| {
                        format!(r#"Failed to list users of profile "{profile_name}""#)
                    })?
                }
            }
            UserAction::Info { usernames } => info_user(config_dir, &config, profile, usernames)
                .wrap_err_with(|| {
                    format!(r#"Failed while querying users of profile "{profile_name}""#)
                })?,
            UserAction::New { usernames, days } => {
                new_user(config_dir, &config, profile, usernames, *days, force).wrap_err_with(
                    || format!(r#"Failed while adding users to profile "{profile_name}""#),
                )?
            }
            UserAction::Remove { usernames } => {
                remove_user(config_dir, &config, profile, usernames, force).wrap_err_with(|| {
                    format!(r#"Failed while removing users from profile "{profile_name}""#)
                })?
            }
            UserAction::Package {
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
                    usernames,
                    *add_prefix,
                    output_dir,
                    force,
                    *keep_temp,
                )
                .wrap_err_with(|| {
                    format!(r#"Failed while packaging users of profile "{profile_name}""#)
                })?
            }
        },
    }

    // post-action scripts
    if !no_post_action_scripts {
        run_post_action_scripts(profile, &action)?;
    }

    Ok(())
}

fn run_post_action_scripts(profile: &Profile, action: &Action) -> color_eyre::Result<()> {
    let Ok(action_kind) = action.try_into() else {
        // action does not support scripting
        return Ok(());
    };
    let Some(scripts) = &profile.post_action_scripts else {
        // no scripts specified
        return Ok(());
    };

    scripts
        .run_for(action_kind)
        .wrap_err("Failed while running post-action scripts")?;
    Ok(())
}

mod cli;
mod config;

use clap::Parser;
use simplelog::{ColorChoice, TermLogger, TerminalMode};

use crate::{
    cli::{Action, CliArgs},
    config::{default_config_path, Config},
};

fn main() -> color_eyre::Result<()> {
    // install panic & error report handlers
    color_eyre::install()?;

    // parse CLI
    let CliArgs { config_path, action, verbosity } = CliArgs::parse();

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

    // actions
    match action {
        Action::List => todo!(),
        Action::NewUser { username, days } => todo!(),
        Action::RmUser { username, no_update_crl } => todo!(),
        Action::PackageFor { usernames, add_prefix, output_dir } => todo!(),
    }

    Ok(())
}

mod shared;

use std::path::Path;

use itertools::Itertools;

use crate::{action::shared::get_users, config::Profile};

pub fn list_users(config_dir: impl AsRef<Path>, profile: &Profile) -> color_eyre::Result<()> {
    let output = get_users(config_dir, profile)?.into_iter().join("\n");
    println!("{output}");
    Ok(())
}

use std::{str::FromStr, sync::LazyLock};

use color_eyre::eyre::OptionExt;
use regex::Regex;

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

use std::{collections::BTreeMap, ffi::OsStr, path::Path, str::FromStr, sync::LazyLock};

use color_eyre::eyre::OptionExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::cli::ActionType;

/// A validated username.
#[derive(Clone, Debug, derive_more::Deref, derive_more::Display, Eq, PartialEq)]
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
/// Required by xshell.
impl AsRef<OsStr> for Username {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(&self.0)
    }
}
/// Required by path concatenation.
impl AsRef<Path> for Username {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

/// A map of custom scripts to be run before or after a particular action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CustomScriptsMap(BTreeMap<ActionType, Vec<String>>);
impl Default for CustomScriptsMap {
    fn default() -> Self {
        let map = ActionType::iter().map(|a| (a, vec![])).collect();
        Self(map)
    }
}

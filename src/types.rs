use std::{collections::BTreeMap, ffi::OsStr, path::Path, str::FromStr, sync::LazyLock};

use color_eyre::eyre::{bail, OptionExt};
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

/// A validated map of custom scripts to be run before or after a particular action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CustomScriptsMapValidator")]
pub struct CustomScriptsMap(BTreeMap<ActionType, Vec<String>>);
impl Default for CustomScriptsMap {
    fn default() -> Self {
        let map = ActionType::iter()
            .filter(|a| !matches!(a, ActionType::InitConfig)) // non-applicable subcommands
            .map(|a| (a, vec![]))
            .collect();
        Self(map)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct CustomScriptsMapValidator(BTreeMap<ActionType, Vec<String>>);
impl TryFrom<CustomScriptsMapValidator> for CustomScriptsMap {
    type Error = color_eyre::Report;

    fn try_from(scripts: CustomScriptsMapValidator) -> Result<Self, Self::Error> {
        let scripts = scripts.0;

        // non-applicable subcommands must not be defined
        for action in [ActionType::InitConfig] {
            if scripts.get(&action).is_some() {
                bail!(r#"Custom scripts are not supported for the "{action}" subcommand"#);
            }
        }

        Ok(Self(scripts))
    }
}

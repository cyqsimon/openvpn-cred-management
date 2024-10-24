use std::{collections::BTreeMap, ffi::OsStr, path::Path, str::FromStr, sync::LazyLock};

use color_eyre::eyre::{bail, eyre, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use xshell::{cmd, Shell};

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
            .ok_or_else(|| eyre!(r#"Username "{s}" does not match "{REGEX}""#))
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
impl CustomScriptsMap {
    /// Return an example map.
    pub fn example() -> Self {
        let mut map = Self::default();

        // insert example scripts
        map.0
            .entry(ActionType::List)
            .or_default()
            .push("echo 'Never play f6' >/dev/stderr".into());

        map
    }

    /// Run all custom scripts defined for a type of action.
    ///
    /// The scripts are run in the current working directory.
    pub fn run_for(&self, action: ActionType) -> color_eyre::Result<()> {
        // skip if map key is not found or if the map entry is empty
        let Some(scripts) = self
            .0
            .get(&action)
            .and_then(|v| (!v.is_empty()).then_some(v))
        else {
            return Ok(());
        };

        let sh = Shell::new().wrap_err("Failed to create subshell")?;
        for script in scripts {
            cmd!(sh, "bash -c {script}")
                .run()
                .wrap_err("A custom script failed to execute")?;
        }

        Ok(())
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
            if scripts.contains_key(&action) {
                bail!(r#"Custom scripts are not supported for the "{action}" subcommand"#);
            }
        }

        Ok(Self(scripts))
    }
}

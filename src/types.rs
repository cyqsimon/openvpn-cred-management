use std::{collections::BTreeMap, ffi::OsStr, path::Path, str::FromStr, sync::LazyLock};

use color_eyre::eyre::{bail, eyre, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use xshell::{cmd, Shell};

use crate::cli::Action;

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

#[allow(clippy::enum_variant_names)]
/// A known action that supports custom scripting.
#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    strum::EnumIter,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum ScriptableActionKind {
    UserList,
    UserInfo,
    UserNew,
    UserRm,
    UserPkg,
}
impl TryFrom<&Action> for ScriptableActionKind {
    type Error = color_eyre::Report;
    fn try_from(action: &Action) -> Result<Self, Self::Error> {
        use crate::cli::{GenAction as G, ProfileAction as P, UserAction as U};

        // don't use wildcard matching here, so that the compiler will complain
        // if we added an action but forgot to update this
        let kind = match action {
            Action::Gen { action: G::Completion { .. } | G::Config }
            | Action::Profile { action: P::List } => {
                bail!("This action is not scriptable")
            }
            Action::User { action, .. } => match action {
                U::List { .. } => Self::UserList,
                U::Info { .. } => Self::UserInfo,
                U::New { .. } => Self::UserNew,
                U::Rm { .. } => Self::UserRm,
                U::Pkg { .. } => Self::UserPkg,
            },
        };
        Ok(kind)
    }
}
impl TryFrom<Action> for ScriptableActionKind {
    type Error = color_eyre::Report;
    fn try_from(action: Action) -> Result<Self, Self::Error> {
        (&action).try_into()
    }
}

/// A map of custom scripts to be run before or after a particular action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CustomScriptsMap(BTreeMap<ScriptableActionKind, Vec<String>>);
impl Default for CustomScriptsMap {
    fn default() -> Self {
        let map = ScriptableActionKind::iter().map(|a| (a, vec![])).collect();
        Self(map)
    }
}
impl CustomScriptsMap {
    /// Return an example map.
    pub fn example() -> Self {
        let mut map = Self::default();

        // insert example scripts
        map.0
            .entry(ScriptableActionKind::UserList)
            .or_default()
            .push("echo 'Never play f6' >/dev/stderr".into());

        map
    }

    /// Run all custom scripts defined for a kind of action.
    ///
    /// The scripts are run in the current working directory.
    pub fn run_for(&self, action: ScriptableActionKind) -> color_eyre::Result<()> {
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
                .run_interactive()
                .wrap_err("A custom script failed to execute")?;
        }

        Ok(())
    }
}

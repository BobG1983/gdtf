//! The newtypes a launch recipe is written in — package, features, working directory, and
//! environment overrides (GTW-875).
//!
//! Every one of these is a domain value the launcher passes to `cargo run`, so none of
//! them is a bare `String` / `PathBuf` / `Vec` (no-bare-types). [`FeatureList`] also owns
//! the one piece of formatting knowledge in the set: how a list of features is rendered
//! for `cargo --features` (comma-joined, no spaces).

use core::ops::Deref;
use std::path::{Path, PathBuf};

/// The cargo **package** a launch builds and runs — the `-p` value.
///
/// Private-inner newtype over `String` (no-bare-types). `grimdark_turfwar` for the game;
/// the editor package once an editor launch exists (GTW-808).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoPackage(String);

impl CargoPackage {
    /// Build a package value from its cargo package name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for CargoPackage {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One cargo **feature** name a launch enables, e.g. `dev_tools`.
///
/// Private-inner newtype over `String` (no-bare-types).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureName(String);

impl FeatureName {
    /// Build a feature name from its text.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for FeatureName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The ordered set of features a launch enables.
///
/// Private-inner newtype over `Vec<FeatureName>` (no-bare-types). It owns the rendering
/// rule for `cargo --features`: comma-joined with no spaces. An EMPTY list means "pass no
/// `--features` flag at all", which is not the same as passing an empty string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FeatureList(Vec<FeatureName>);

impl FeatureList {
    /// Build a feature list from its names, in the order they were given.
    #[must_use]
    pub const fn new(features: Vec<FeatureName>) -> Self {
        Self(features)
    }

    /// The `--features` argument value: the names comma-joined, no spaces.
    ///
    /// Returns `None` when the list is empty, so the caller omits the flag entirely.
    #[must_use]
    pub fn render(&self) -> Option<String> {
        if self.0.is_empty() {
            return None;
        }
        Some(
            self.0
                .iter()
                .map(|feature| feature.as_str())
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

impl Deref for FeatureList {
    type Target = [FeatureName];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The **directory the launcher runs in** — which checkout `cargo run` builds.
///
/// Private-inner newtype over `PathBuf` (no-bare-types). This is the value that decides
/// WHICH TREE is under test: without it a launch inherits the MCP host's own directory
/// and silently builds the main checkout even when the code under review lives in a git
/// worktree (GTW-875).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkingDir(PathBuf);

impl WorkingDir {
    /// Build a working directory from its path.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Deref for WorkingDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The **name** of an environment variable a launch sets on the child.
///
/// Private-inner newtype over `String` (no-bare-types).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Build a variable name from its text.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for EnvVarName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The **value** a launch sets an environment variable to.
///
/// Private-inner newtype over `String` (no-bare-types).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarValue(String);

impl EnvVarValue {
    /// Build a variable value from its text.
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }
}

impl Deref for EnvVarValue {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One name/value pair set on the launched child.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVar {
    /// The variable's name.
    name:  EnvVarName,
    /// The value it is set to.
    value: EnvVarValue,
}

impl EnvVar {
    /// Build a name/value pair.
    #[must_use]
    pub const fn new(name: EnvVarName, value: EnvVarValue) -> Self {
        Self { name, value }
    }

    /// The variable's name.
    #[must_use]
    pub const fn name(&self) -> &EnvVarName {
        &self.name
    }

    /// The value it is set to.
    #[must_use]
    pub const fn value(&self) -> &EnvVarValue {
        &self.value
    }
}

/// The environment variables a launch sets on the child, in the order they were given.
///
/// Private-inner newtype over `Vec<EnvVar>` (no-bare-types). These are ADDITIONS to the
/// inherited environment — the gate variables a dev build reads, such as
/// `GDTF_BATTLE_SEED`, which no hardcoded launcher could pass through (GTW-875).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EnvOverrides(Vec<EnvVar>);

impl EnvOverrides {
    /// Build the override set from its pairs.
    #[must_use]
    pub const fn new(vars: Vec<EnvVar>) -> Self {
        Self(vars)
    }
}

impl Deref for EnvOverrides {
    type Target = [EnvVar];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

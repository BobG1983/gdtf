//! Value types used in a launch recipe.

use core::ops::Deref;
use std::path::{Path, PathBuf};

/// Cargo package name to run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoPackage(String);

impl CargoPackage {
    /// Wrap a package name.
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

/// Cargo profile to build and run under, as `--profile` takes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoProfile(String);

impl CargoProfile {
    /// Wrap a profile name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// The release profile, which builds with `debug_assertions` off.
    #[must_use]
    pub fn release() -> Self {
        Self::new("release".to_owned())
    }
}

impl Deref for CargoProfile {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One cargo feature name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureName(String);

impl FeatureName {
    /// Wrap a feature name.
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

/// Ordered list of features for `--features`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FeatureList(Vec<FeatureName>);

impl FeatureList {
    /// From a list of names.
    #[must_use]
    pub const fn new(features: Vec<FeatureName>) -> Self {
        Self(features)
    }

    /// Comma-joined string, or `None` if empty.
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

/// Working directory for the child process.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkingDir(PathBuf);

impl WorkingDir {
    /// Wrap a path.
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

/// Environment variable name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Wrap a name.
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

/// Environment variable value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarValue(String);

impl EnvVarValue {
    /// Wrap a value.
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

/// One env override pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVar {
    name:  EnvVarName,
    value: EnvVarValue,
}

impl EnvVar {
    /// Build a pair.
    #[must_use]
    pub const fn new(name: EnvVarName, value: EnvVarValue) -> Self {
        Self { name, value }
    }

    /// Variable name.
    #[must_use]
    pub const fn name(&self) -> &EnvVarName {
        &self.name
    }

    /// Variable value.
    #[must_use]
    pub const fn value(&self) -> &EnvVarValue {
        &self.value
    }
}

/// Extra env vars applied before channel vars.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EnvOverrides(Vec<EnvVar>);

impl EnvOverrides {
    /// From a list of pairs.
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

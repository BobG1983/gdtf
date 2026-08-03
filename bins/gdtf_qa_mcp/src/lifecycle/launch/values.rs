use core::ops::Deref;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoPackage(String);

impl CargoPackage {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureName(String);

impl FeatureName {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FeatureList(Vec<FeatureName>);

impl FeatureList {
        #[must_use]
    pub const fn new(features: Vec<FeatureName>) -> Self {
        Self(features)
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkingDir(PathBuf);

impl WorkingDir {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarName(String);

impl EnvVarName {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVarValue(String);

impl EnvVarValue {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvVar {
        name:  EnvVarName,
        value: EnvVarValue,
}

impl EnvVar {
        #[must_use]
    pub const fn new(name: EnvVarName, value: EnvVarValue) -> Self {
        Self { name, value }
    }

        #[must_use]
    pub const fn name(&self) -> &EnvVarName {
        &self.name
    }

        #[must_use]
    pub const fn value(&self) -> &EnvVarValue {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EnvOverrides(Vec<EnvVar>);

impl EnvOverrides {
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

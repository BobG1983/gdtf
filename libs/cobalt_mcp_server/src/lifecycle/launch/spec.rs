//! Full launch recipe for a host process.

use super::{
    channel::McpChannel,
    values::{CargoPackage, CargoProfile, EnvOverrides, FeatureList, WorkingDir},
};

/// Package, features, profile, cwd, env, and channel for one launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    package:     CargoPackage,
    features:    FeatureList,
    profile:     Option<CargoProfile>,
    working_dir: Option<WorkingDir>,
    env:         EnvOverrides,
    channel:     McpChannel,
}

impl LaunchSpec {
    /// Build a recipe from parts.
    #[must_use]
    pub const fn new(
        package: CargoPackage,
        features: FeatureList,
        working_dir: Option<WorkingDir>,
        env: EnvOverrides,
        channel: McpChannel,
    ) -> Self {
        Self {
            package,
            features,
            profile: None,
            working_dir,
            env,
            channel,
        }
    }

    /// The same recipe built and run under `profile`.
    #[must_use]
    pub fn with_profile(self, profile: Option<CargoProfile>) -> Self {
        Self { profile, ..self }
    }

    /// Cargo profile, or `None` for cargo's own default.
    #[must_use]
    pub const fn profile(&self) -> Option<&CargoProfile> {
        self.profile.as_ref()
    }

    /// Cargo package name.
    #[must_use]
    pub const fn package(&self) -> &CargoPackage {
        &self.package
    }

    /// Feature list.
    #[must_use]
    pub const fn features(&self) -> &FeatureList {
        &self.features
    }

    /// Optional working directory.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&WorkingDir> {
        self.working_dir.as_ref()
    }

    /// Extra environment variables.
    #[must_use]
    pub const fn env(&self) -> &EnvOverrides {
        &self.env
    }

    /// MCP channel env names.
    #[must_use]
    pub const fn channel(&self) -> &McpChannel {
        &self.channel
    }

    /// Explicit cwd, or the current process cwd.
    #[must_use]
    pub fn resolved_working_dir(&self) -> Option<WorkingDir> {
        self.working_dir
            .clone()
            .or_else(|| std::env::current_dir().ok().map(WorkingDir::new))
    }

    /// Whether two recipes would launch the same way.
    #[must_use]
    pub fn is_same_launch_as(&self, other: &Self) -> bool {
        self.package == other.package
            && self.features == other.features
            && self.profile == other.profile
            && self.env == other.env
            && self.channel == other.channel
            && self.resolved_working_dir() == other.resolved_working_dir()
    }
}

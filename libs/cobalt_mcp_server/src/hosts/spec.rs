//! One registered host, as a value the whole bridge reads.

use cobalt_mcp_protocol::ports::McpPort;

use super::name::HostName;
use crate::lifecycle::{
    CargoPackage, FeatureList, LaunchPolicy, LaunchSpec, LifecycleConfig, WorkingDir,
};

/// Everything the bridge needs to launch and address one host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHostSpec {
    name:         HostName,
    package:      CargoPackage,
    features:     FeatureList,
    default_port: McpPort,
    working_dir:  Option<WorkingDir>,
    lifecycle:    LifecycleConfig,
}

impl McpHostSpec {
    /// Register a host from its own parts.
    #[must_use]
    pub const fn new(
        name: HostName,
        package: CargoPackage,
        features: FeatureList,
        default_port: McpPort,
        working_dir: Option<WorkingDir>,
        lifecycle: LifecycleConfig,
    ) -> Self {
        Self {
            name,
            package,
            features,
            default_port,
            working_dir,
            lifecycle,
        }
    }

    /// Name this host answers to.
    #[must_use]
    pub const fn name(&self) -> &HostName {
        &self.name
    }

    /// Cargo package a launch builds and runs.
    #[must_use]
    pub const fn package(&self) -> &CargoPackage {
        &self.package
    }

    /// Cargo features a launch enables.
    #[must_use]
    pub const fn features(&self) -> &FeatureList {
        &self.features
    }

    /// Port this host listens on when nothing overrides it.
    #[must_use]
    pub const fn default_port(&self) -> McpPort {
        self.default_port
    }

    /// Directory a launch builds in, or `None` for the bridge's own.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&WorkingDir> {
        self.working_dir.as_ref()
    }

    /// Boot, kill and sweep timing, with the launch policy.
    #[must_use]
    pub const fn lifecycle_config(&self) -> LifecycleConfig {
        self.lifecycle
    }

    /// Whether a launch reuses the recorded child or starts another.
    #[must_use]
    pub const fn launch_policy(&self) -> LaunchPolicy {
        self.lifecycle.launch_policy()
    }

    /// Whether several children of this host can be up at once.
    #[must_use]
    pub const fn runs_many_instances(&self) -> bool {
        matches!(self.launch_policy(), LaunchPolicy::AlwaysSpawn)
    }

    /// Launch recipe this host uses when a call overrides nothing.
    #[must_use]
    pub fn default_spec(&self) -> LaunchSpec {
        LaunchSpec::new(
            self.package.clone(),
            self.features.clone(),
            self.working_dir.clone(),
        )
    }

    /// How an error message names this host's stop call.
    #[must_use]
    pub fn stop_tool_name(&self) -> String {
        format!("stop(host={:?})", self.name.as_str())
    }

    /// How an error message names this host's launch call.
    #[must_use]
    pub fn launch_tool_name(&self) -> String {
        format!("launch(host={:?})", self.name.as_str())
    }
}

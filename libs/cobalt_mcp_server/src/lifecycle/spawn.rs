//! Spawn host processes via `cargo run`.

use std::{
    io,
    process::{Command, Stdio},
};

use cobalt_mcp_protocol::ports::McpPort;

use super::{
    child::{ManagedChild, ProcessChild},
    launch::LaunchSpec,
};

/// How to spawn a managed child for a launch recipe.
pub trait ChildSpawner {
    /// Spawn a child for `port` using `spec`.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying process spawn.
    fn spawn(&self, port: McpPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>>;
}

/// Spawner that runs `cargo run -p …`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CargoSpawner;

impl CargoSpawner {
    /// Create a cargo spawner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Build the `cargo run` command for a recipe and port.
#[must_use]
pub fn build_command(port: McpPort, spec: &LaunchSpec) -> Command {
    let mut command = Command::new("cargo");
    command.args(["run", "-p", spec.package().as_str()]);
    if let Some(profile) = spec.profile() {
        command.args(["--profile", profile.as_str()]);
    }
    if let Some(features) = spec.features().render() {
        command.args(["--features", features.as_str()]);
    }
    if let Some(dir) = spec.working_dir() {
        command.current_dir(&**dir);
    }
    for var in spec.env().iter() {
        command.env(var.name().as_str(), var.value().as_str());
    }
    // After the overrides, so an `env` argument cannot displace the launch port.
    command.env(spec.channel().port().as_str(), (*port).to_string());
    command.stdin(Stdio::null());
    command
}

impl ChildSpawner for CargoSpawner {
    fn spawn(&self, port: McpPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        ProcessChild::spawn(build_command(port, spec))
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::PathBuf};

    use super::{McpPort, build_command};
    use crate::{
        hosts::test_support::registered,
        lifecycle::launch::{
            CargoPackage, CargoProfile, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList,
            FeatureName, LaunchSpec, WorkingDir,
        },
    };

    // The recipe a host registered under `name` launches with, and the port it listens on.
    fn recipe_of(name: &str, port: u16, many_instances: bool) -> (LaunchSpec, McpPort) {
        let host = registered(name, port, many_instances);
        (host.default_spec(), host.default_port())
    }

    fn args_of(command: &std::process::Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn env_of(command: &std::process::Command, name: &str) -> Option<String> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().into_owned())
    }

    #[test]
    fn a_registered_hosts_recipe_matches_a_plain_cargo_launch() {
        let (spec, port) = recipe_of("alpha", 4100, false);
        let command = build_command(port, &spec);
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "alpha_package".to_owned(),
                "--features".to_owned(),
                "alpha_feature".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "ALPHA_CHANNEL"), None);
        assert_eq!(
            env_of(&command, "ALPHA_CHANNEL_PORT"),
            Some("4100".to_owned())
        );
        assert!(command.get_current_dir().is_none());
    }

    #[test]
    fn a_release_child_is_built_and_run_under_the_release_profile() {
        let (spec, port) = recipe_of("alpha", 4100, false);
        let command = build_command(port, &spec.with_profile(Some(CargoProfile::release())));
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "alpha_package".to_owned(),
                "--profile".to_owned(),
                "release".to_owned(),
                "--features".to_owned(),
                "alpha_feature".to_owned(),
            ]
        );
    }

    #[test]
    fn the_child_is_handed_the_launch_port() {
        let (spec, _) = recipe_of("beta", 4200, true);
        let command = build_command(McpPort::new(4220), &spec);
        assert_eq!(
            env_of(&command, spec.channel().port().as_str()),
            Some("4220".to_owned())
        );
    }

    #[test]
    fn the_launch_port_outranks_an_env_override_of_the_same_name() {
        let (default, _) = recipe_of("beta", 4200, true);
        let port_var = default.channel().port().as_str().to_owned();
        let spec = LaunchSpec::new(
            default.package().clone(),
            default.features().clone(),
            None,
            EnvOverrides::new(vec![EnvVar::new(
                EnvVarName::new(port_var.clone()),
                EnvVarValue::new("7999".to_owned()),
            )]),
            default.channel().clone(),
        );
        let command = build_command(McpPort::new(4220), &spec);
        assert_eq!(env_of(&command, &port_var), Some("4220".to_owned()));
    }

    #[test]
    fn recipe_drives_package_features_directory_and_env() {
        let (default, _) = recipe_of("alpha", 4100, false);
        let spec = LaunchSpec::new(
            CargoPackage::new("another_package".to_owned()),
            FeatureList::new(vec![
                FeatureName::new("dynamic_linking".to_owned()),
                FeatureName::new("dev_tools".to_owned()),
            ]),
            Some(WorkingDir::new(PathBuf::from("/tmp/a-worktree"))),
            EnvOverrides::new(vec![EnvVar::new(
                EnvVarName::new("SAMPLE_SEED".to_owned()),
                EnvVarValue::new("42".to_owned()),
            )]),
            default.channel().clone(),
        );
        let command = build_command(McpPort::new(4321), &spec);
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "another_package".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,dev_tools".to_owned(),
            ]
        );
        assert_eq!(
            command.get_current_dir().and_then(std::path::Path::to_str),
            Some("/tmp/a-worktree")
        );
        assert_eq!(env_of(&command, "SAMPLE_SEED"), Some("42".to_owned()));
    }

    #[test]
    fn a_second_registered_host_runs_its_own_package() {
        let (spec, port) = recipe_of("beta", 4200, true);
        let command = build_command(port, &spec);
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "beta_package".to_owned(),
                "--features".to_owned(),
                "beta_feature".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "BETA_CHANNEL"), None);
        assert_eq!(env_of(&command, "ALPHA_CHANNEL"), None);
    }
}

//! Spawn host processes via `cargo run`.

use std::{
    io,
    process::{Command, Stdio},
};

use cobalt_mcp_protocol::ports::{MCP_PORT_FLAG, McpPort};

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
///
/// The child is handed no environment of its own; the port goes on its command line.
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
    command.args(["--", MCP_PORT_FLAG, &(*port).to_string()]);
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
    use std::path::PathBuf;

    use super::{McpPort, build_command};
    use crate::{
        hosts::test_support::registered,
        lifecycle::launch::{
            CargoPackage, CargoProfile, FeatureList, FeatureName, LaunchSpec, WorkingDir,
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

    // The trailing arguments cargo passes on to the child, naming the port it binds.
    fn port_arguments(port: &str) -> Vec<String> {
        vec!["--".to_owned(), "--mcp-port".to_owned(), port.to_owned()]
    }

    #[test]
    fn a_registered_hosts_recipe_matches_a_plain_cargo_launch() {
        let (spec, port) = recipe_of("alpha", 4100, false);
        let command = build_command(port, &spec);
        assert_eq!(
            args_of(&command),
            [
                vec![
                    "run".to_owned(),
                    "-p".to_owned(),
                    "alpha_package".to_owned(),
                    "--features".to_owned(),
                    "alpha_feature".to_owned(),
                ],
                port_arguments("4100"),
            ]
            .concat()
        );
        assert!(command.get_current_dir().is_none());
    }

    #[test]
    fn a_release_child_is_built_and_run_under_the_release_profile() {
        let (spec, port) = recipe_of("alpha", 4100, false);
        let command = build_command(port, &spec.with_profile(Some(CargoProfile::release())));
        assert_eq!(
            args_of(&command),
            [
                vec![
                    "run".to_owned(),
                    "-p".to_owned(),
                    "alpha_package".to_owned(),
                    "--profile".to_owned(),
                    "release".to_owned(),
                    "--features".to_owned(),
                    "alpha_feature".to_owned(),
                ],
                port_arguments("4100"),
            ]
            .concat()
        );
    }

    #[test]
    fn the_child_is_handed_the_launch_port() {
        let (spec, _) = recipe_of("beta", 4200, true);
        let command = build_command(McpPort::new(4220), &spec);
        let args = args_of(&command);
        assert_eq!(
            args.get(args.len().saturating_sub(3)..),
            Some(port_arguments("4220").as_slice()),
            "the launch port ends the command line, not the recipe's own default: {args:?}"
        );
    }

    #[test]
    fn the_child_is_handed_no_variable_and_takes_its_port_on_the_command_line() {
        let (default, _) = recipe_of("beta", 4200, true);
        let command = build_command(McpPort::new(4220), &default);
        let handed: Vec<String> = command
            .get_envs()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        assert!(
            handed.is_empty(),
            "a recipe carries no environment of its own, and the spawner sets none: {handed:?}"
        );
        let args = args_of(&command);
        assert_eq!(
            args.get(args.len().saturating_sub(3)..),
            Some(port_arguments("4220").as_slice()),
            "the launch port reaches the child as an argument instead: {args:?}"
        );
    }

    #[test]
    fn recipe_drives_package_features_and_directory() {
        let spec = LaunchSpec::new(
            CargoPackage::new("another_package".to_owned()),
            FeatureList::new(vec![
                FeatureName::new("dynamic_linking".to_owned()),
                FeatureName::new("dev_tools".to_owned()),
            ]),
            Some(WorkingDir::new(PathBuf::from("/tmp/a-worktree"))),
        );
        let command = build_command(McpPort::new(4321), &spec);
        assert_eq!(
            args_of(&command),
            [
                vec![
                    "run".to_owned(),
                    "-p".to_owned(),
                    "another_package".to_owned(),
                    "--features".to_owned(),
                    "dynamic_linking,dev_tools".to_owned(),
                ],
                port_arguments("4321"),
            ]
            .concat()
        );
        assert_eq!(
            command.get_current_dir().and_then(std::path::Path::to_str),
            Some("/tmp/a-worktree")
        );
    }

    #[test]
    fn a_second_registered_host_runs_its_own_package() {
        let (spec, port) = recipe_of("beta", 4200, true);
        let command = build_command(port, &spec);
        assert_eq!(
            args_of(&command),
            [
                vec![
                    "run".to_owned(),
                    "-p".to_owned(),
                    "beta_package".to_owned(),
                    "--features".to_owned(),
                    "beta_feature".to_owned(),
                ],
                port_arguments("4200"),
            ]
            .concat()
        );
    }
}

//! Spawn host processes via `cargo run`.

use std::{
    io,
    process::{Command, Stdio},
};

use super::{
    child::{ManagedChild, ProcessChild},
    launch::LaunchSpec,
};
use crate::link::QaPort;

/// How to spawn a managed child for a launch recipe.
pub trait ChildSpawner {
    /// Spawn a child for `port` using `spec`.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying process spawn.
    fn spawn(&self, port: QaPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>>;
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
pub fn build_command(port: QaPort, spec: &LaunchSpec) -> Command {
    let mut command = Command::new("cargo");
    command.args(["run", "-p", spec.package().as_str()]);
    if let Some(features) = spec.features().render() {
        command.args(["--features", features.as_str()]);
    }
    if let Some(dir) = spec.working_dir() {
        command.current_dir(&**dir);
    }
    for var in spec.env().iter() {
        command.env(var.name().as_str(), var.value().as_str());
    }
    // Hosts listen on shared protocol ports in debug builds; nothing to pass through.
    let _ = (port, spec.channel());
    command.stdin(Stdio::null());
    command
}

impl ChildSpawner for CargoSpawner {
    fn spawn(&self, port: QaPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        ProcessChild::spawn(build_command(port, spec))
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::PathBuf};

    use super::{QaPort, build_command};
    use crate::lifecycle::launch::{
        EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName, LaunchSpec,
        QaChannel, WorkingDir,
    };

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
    fn default_recipe_matches_plain_git_launch() {
        let command = build_command(QaPort::new(7616), &LaunchSpec::game_default());
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "grimdark_turfwar".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,dev_tools".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "GDTF_NET_QA"), None);
        assert_eq!(env_of(&command, "GDTF_NET_QA_PORT"), None);
        assert!(command.get_current_dir().is_none());
    }

    #[test]
    fn recipe_drives_package_features_directory_and_env() {
        let spec = LaunchSpec::new(
            crate::lifecycle::launch::CargoPackage::new("gdtf_content_editor".to_owned()),
            FeatureList::new(vec![
                FeatureName::new("dynamic_linking".to_owned()),
                FeatureName::new("dev_tools".to_owned()),
            ]),
            Some(WorkingDir::new(PathBuf::from("/tmp/a-worktree"))),
            EnvOverrides::new(vec![EnvVar::new(
                EnvVarName::new("GDTF_BATTLE_SEED".to_owned()),
                EnvVarValue::new("42".to_owned()),
            )]),
            QaChannel::game(),
        );
        let command = build_command(QaPort::new(4321), &spec);
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "gdtf_content_editor".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,dev_tools".to_owned(),
            ]
        );
        assert_eq!(
            command.get_current_dir().and_then(std::path::Path::to_str),
            Some("/tmp/a-worktree")
        );
        assert_eq!(env_of(&command, "GDTF_BATTLE_SEED"), Some("42".to_owned()));
    }

    #[test]
    fn editor_recipe_runs_the_editor_binary() {
        let command = build_command(QaPort::new(7617), &LaunchSpec::editor_default());
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "gdtf_content_editor_bin".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,file_watcher".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "GDTF_EDITOR_NET_QA"), None);
        assert_eq!(env_of(&command, "GDTF_NET_QA"), None);
    }
}

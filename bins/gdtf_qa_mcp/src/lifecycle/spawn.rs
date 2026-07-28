//! How the game child is launched — the [`ChildSpawner`] trait and its real
//! [`CargoSpawner`] (GTW-745, parameterized in GTW-875).
//!
//! [`ChildSpawner`] is the one piece the lifecycle manager takes as a dependency, so a test
//! can supply a stub that launches a harmless placeholder process while every other part
//! of the launch / stop logic runs unchanged. [`CargoSpawner`] is the real one: it runs
//! `cargo run` for whatever [`LaunchSpec`] the caller hands it, with the QA environment
//! set on top.
//!
//! The recipe arrives PER LAUNCH rather than being fixed when the spawner is built, so a
//! single running MCP host can launch a plain build, then a `dev_tools` build, then a
//! build in a git worktree, without being restarted (GTW-875) — and, since the recipe
//! also names the two QA-channel variables, the SAME spawner launches the game and the
//! editor with no per-host branch (GTW-808).

use std::{
    io,
    process::{Command, Stdio},
};

use super::{
    child::{ManagedChild, ProcessChild},
    launch::LaunchSpec,
};
use crate::link::QaPort;

/// Launches a child bound to a chosen port, following a launch recipe.
///
/// The manager depends on this trait, not on [`CargoSpawner`], so the readiness / timeout
/// / stop logic can be driven against a stub process in tests without launching the real
/// game or editor binary.
pub trait ChildSpawner {
    /// Spawn the child described by `spec`, telling it to listen on `port`.
    ///
    /// # Errors
    ///
    /// The underlying [`io::Error`] if the child process cannot be spawned.
    fn spawn(&self, port: QaPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>>;
}

/// The real spawner — runs `cargo run` for the recipe it is given, with the QA environment
/// set.
///
/// The child's stdout is discarded (the MCP host's own stdout is the JSON-RPC channel and
/// must not be polluted); its stderr is captured by [`ProcessChild`] for the failure tail.
#[derive(Debug, Clone, Copy, Default)]
pub struct CargoSpawner;

impl CargoSpawner {
    /// Build the real spawner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Assemble the `cargo run` command for `spec` on `port`.
///
/// The recipe's own environment overrides are applied FIRST and the two QA variables it
/// names LAST, so a caller can pass through a dev gate such as `GDTF_BATTLE_SEED` but can
/// never displace the channel and port the host is about to probe for readiness. WHICH
/// two variables those are comes from the recipe's [`QaChannel`](super::launch::QaChannel)
/// — the game's `GDTF_NET_QA` pair or the editor's `GDTF_EDITOR_NET_QA` pair (GTW-808).
///
/// Split out from [`CargoSpawner::spawn`] so the assembled command is inspectable — the
/// unit tests read back the arguments, directory, and environment this produces rather
/// than launching a real build.
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
    command
        .env(spec.channel().enable().as_str(), "1")
        .env(spec.channel().port().as_str(), format!("{}", *port))
        .stdin(Stdio::null())
        .stdout(Stdio::null());
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

    /// The command's arguments, as owned strings, for readable assertions.
    fn args_of(command: &std::process::Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    /// The value the command sets for `name`, if it sets one.
    fn env_of(command: &std::process::Command, name: &str) -> Option<String> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().into_owned())
    }

    /// The default recipe reproduces the launch the spawner used to hardcode.
    #[test]
    fn default_recipe_matches_the_previous_hardcoded_launch() {
        let command = build_command(QaPort::new(7616), &LaunchSpec::game_default());
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "grimdark_turfwar".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,net_qa".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "GDTF_NET_QA"), Some("1".to_owned()));
        assert_eq!(
            env_of(&command, "GDTF_NET_QA_PORT"),
            Some("7616".to_owned())
        );
        assert!(command.get_current_dir().is_none());
    }

    /// A recipe naming a package, features, a directory, and environment overrides puts
    /// all four onto the command.
    #[test]
    fn recipe_drives_package_features_directory_and_env() {
        let spec = LaunchSpec::new(
            crate::lifecycle::launch::CargoPackage::new("gdtf_content_editor".to_owned()),
            FeatureList::new(vec![
                FeatureName::new("dynamic_linking".to_owned()),
                FeatureName::new("net_qa".to_owned()),
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
                "dynamic_linking,net_qa,dev_tools".to_owned(),
            ]
        );
        assert_eq!(
            command.get_current_dir().and_then(std::path::Path::to_str),
            Some("/tmp/a-worktree")
        );
        assert_eq!(env_of(&command, "GDTF_BATTLE_SEED"), Some("42".to_owned()));
        assert_eq!(
            env_of(&command, "GDTF_NET_QA_PORT"),
            Some("4321".to_owned())
        );
    }

    /// An override naming a QA variable cannot displace it: the host's channel and port
    /// are applied last and win.
    #[test]
    fn qa_environment_wins_over_an_override_of_the_same_name() {
        let spec = LaunchSpec::new(
            LaunchSpec::game_default().package().clone(),
            FeatureList::default(),
            None,
            EnvOverrides::new(vec![EnvVar::new(
                EnvVarName::new("GDTF_NET_QA_PORT".to_owned()),
                EnvVarValue::new("1".to_owned()),
            )]),
            QaChannel::game(),
        );
        let command = build_command(QaPort::new(9001), &spec);
        assert_eq!(
            env_of(&command, "GDTF_NET_QA_PORT"),
            Some("9001".to_owned())
        );
        // An empty feature list passes no `--features` flag at all.
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "grimdark_turfwar".to_owned()
            ]
        );
    }

    /// The DEFAULT EDITOR recipe is GTW-878 clause 1's command, with the EDITOR's own two
    /// QA variables — not the game's. Setting `GDTF_NET_QA` on an editor child would leave
    /// it inert with no port to probe (GTW-808).
    #[test]
    fn editor_recipe_runs_the_editor_binary_with_the_editor_channel() {
        let command = build_command(QaPort::new(7617), &LaunchSpec::editor_default());
        assert_eq!(
            args_of(&command),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "gdtf_content_editor_bin".to_owned(),
                "--features".to_owned(),
                "dynamic_linking,net_qa".to_owned(),
            ]
        );
        assert_eq!(env_of(&command, "GDTF_EDITOR_NET_QA"), Some("1".to_owned()));
        assert_eq!(
            env_of(&command, "GDTF_EDITOR_NET_QA_PORT"),
            Some("7617".to_owned())
        );
        assert_eq!(env_of(&command, "GDTF_NET_QA"), None);
        assert_eq!(env_of(&command, "GDTF_NET_QA_PORT"), None);
    }
}

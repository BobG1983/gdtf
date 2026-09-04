use std::{fs, path::PathBuf, process, time::SystemTime};

use mcp::{
    CargoPackage, CargoSpawner, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList,
    FeatureName, HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, LaunchSpec, QaChannel,
    QaPort, WorkingDir,
};

use crate::support::{fast_config, free_port};

const PROBE_PACKAGE: &str = "recipe_probe";

const PROBE_PORT_MARKER: &str = "recipe-probe-port";

const PROBE_ECHO_MAIN: &str = r#"fn main() {
    let shown = match std::env::var("__PORT_VAR__") {
        Ok(value) => value,
        Err(_) => "absent".to_owned(),
    };
    eprintln!("__MARKER__={shown}");
}
"#;

fn temp_dir_outside_a_workspace(tag: &str) -> PathBuf {
    let nanos = SystemTime::UNIX_EPOCH
        .elapsed()
        .map_or(0, |since| since.as_nanos());
    let path = std::env::temp_dir().join(format!("recipe-{tag}-{}-{nanos}", process::id()));
    let Ok(()) = fs::create_dir_all(&path) else {
        unreachable!("the test can create a temp directory");
    };
    path
}

fn probe_package_with_main(tag: &str, main: &str) -> PathBuf {
    let path = temp_dir_outside_a_workspace(tag);
    let Ok(()) = fs::create_dir_all(path.join("src")) else {
        unreachable!("the test can create the package's src directory");
    };
    let manifest = format!(
        "[workspace]\n[package]\nname = \"{PROBE_PACKAGE}\"\nversion = \"0.0.0\"\n\
         edition = \"2021\"\n"
    );
    let Ok(()) = fs::write(path.join("Cargo.toml"), manifest) else {
        unreachable!("the test can write the probe manifest");
    };
    let Ok(()) = fs::write(path.join("src/main.rs"), main) else {
        unreachable!("the test can write the probe main");
    };
    path
}

fn probe_package_outside_a_workspace(tag: &str) -> PathBuf {
    probe_package_with_main(tag, "fn main() {}\n")
}

// Prints the value the child was handed, or `absent` when the name reaches it unset.
fn probe_main_echoing(var: &str) -> String {
    PROBE_ECHO_MAIN
        .replace("__PORT_VAR__", var)
        .replace("__MARKER__", PROBE_PORT_MARKER)
}

fn stderr_of_a_failed_launch_on(port: QaPort, boot_ms: u64, spec: &LaunchSpec) -> String {
    let mut manager = HostManager::with_config(Box::new(CargoSpawner::new()), fast_config(boot_ms));
    let outcome = manager.launch(port, spec);
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("the child exits without ever answering a QA probe: {outcome:?}");
    };
    tail.as_str().to_owned()
}

fn stderr_of_a_failed_launch(spec: &LaunchSpec) -> String {
    stderr_of_a_failed_launch_on(QaPort::new(free_port()), 30_000, spec)
}

#[test]
fn the_recipe_features_reach_the_real_cargo_command() {
    let dir = probe_package_outside_a_workspace("features");
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::new(vec![FeatureName::new("recipe_no_such_feature".to_owned())]),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        QaChannel::game(),
    );

    let tail = stderr_of_a_failed_launch(&spec);
    assert!(
        tail.contains("recipe_no_such_feature"),
        "cargo saw the recipe's features: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

#[test]
fn the_recipe_environment_reaches_the_real_child() {
    let dir = probe_package_outside_a_workspace("env");
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::new(vec![EnvVar::new(
            EnvVarName::new("CARGO_BUILD_TARGET".to_owned()),
            EnvVarValue::new("recipe-not-a-real-target".to_owned()),
        )]),
        QaChannel::game(),
    );

    let tail = stderr_of_a_failed_launch(&spec);
    assert!(
        tail.contains("recipe-not-a-real-target"),
        "the child ran with the recipe's environment: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

#[test]
fn the_launch_port_reaches_the_real_child_on_the_channel_variable() {
    let channel = QaChannel::editor();
    let port_var = channel.port().as_str().to_owned();
    let dir = probe_package_with_main("port", &probe_main_echoing(&port_var));
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        channel,
    );
    let port = free_port();

    // The probe is compiled before it runs, so the budget covers a cold rustc.
    let tail = stderr_of_a_failed_launch_on(QaPort::new(port), 120_000, &spec);
    assert!(
        tail.contains(&format!("{PROBE_PORT_MARKER}={port}")),
        "the child read {port} back out of {port_var}: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

#[test]
fn the_recipe_working_directory_is_where_cargo_runs() {
    let dir = temp_dir_outside_a_workspace("cwd");
    let spec = LaunchSpec::new(
        CargoPackage::new("game".to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        QaChannel::game(),
    );
    let mut manager = HostManager::with_config(Box::new(CargoSpawner::new()), fast_config(30_000));

    let outcome = manager.launch(QaPort::new(free_port()), &spec);
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("cargo exits at once with no manifest to build: {outcome:?}");
    };
    let Some(shown) = dir.to_str() else {
        unreachable!("the temp directory path is valid UTF-8");
    };
    assert!(
        tail.contains(shown),
        "cargo ran in the recipe's directory and said so: {}",
        tail.as_str()
    );
    drop(fs::remove_dir_all(&dir));
}

#[test]
fn successive_launches_can_name_different_recipes() {
    let first_dir = temp_dir_outside_a_workspace("first");
    let second_dir = temp_dir_outside_a_workspace("second");
    let mut manager = HostManager::with_config(Box::new(CargoSpawner::new()), fast_config(30_000));

    for dir in [&first_dir, &second_dir] {
        let spec = LaunchSpec::new(
            CargoPackage::new("game".to_owned()),
            FeatureList::default(),
            Some(WorkingDir::new(dir.clone())),
            EnvOverrides::default(),
            QaChannel::game(),
        );
        let outcome = manager.launch(QaPort::new(free_port()), &spec);
        let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
            unreachable!("cargo exits at once with no manifest to build: {outcome:?}");
        };
        let Some(shown) = dir.to_str() else {
            unreachable!("the temp directory path is valid UTF-8");
        };
        assert!(
            tail.contains(shown),
            "each launch used its own directory: {}",
            tail.as_str()
        );
    }
    drop(fs::remove_dir_all(&first_dir));
    drop(fs::remove_dir_all(&second_dir));
}

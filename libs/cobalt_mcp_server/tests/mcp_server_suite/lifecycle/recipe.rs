use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

use cobalt_mcp_server::{
    CargoPackage, CargoSpawner, FeatureList, FeatureName, HostLifecycle, HostManager,
    LaunchFailure, LaunchOutcome, LaunchSpec, McpPort, WorkingDir,
};

use crate::lifecycle::support::{
    SAMPLE_PACKAGE, free_port, no_boot_deadline_config, sample_channel,
};

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
    write_probe_package(&path, main);
    path
}

// A probe package at a path that outlives the run, under cargo's tmp dir for this crate's
// tests, so every run after the first finds the probe already built.
fn probe_package_kept_warm(tag: &str, main: &str) -> PathBuf {
    let base =
        std::env::var_os("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let path = base.join(format!("recipe-{tag}"));
    write_probe_package(&path, main);
    path
}

fn write_probe_package(path: &Path, main: &str) {
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

// The child always exits, which `await_readiness` notices before it tests any deadline.
fn stderr_of_a_failed_launch_on(port: McpPort, spec: &LaunchSpec) -> String {
    let mut manager =
        HostManager::with_config(Box::new(CargoSpawner::new()), no_boot_deadline_config());
    let outcome = manager.launch(port, spec);
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("the child exits without ever answering a QA probe: {outcome:?}");
    };
    tail.as_str().to_owned()
}

fn stderr_of_a_failed_launch(spec: &LaunchSpec) -> String {
    stderr_of_a_failed_launch_on(McpPort::new(free_port()), spec)
}

#[test]
fn the_recipe_features_reach_the_real_cargo_command() {
    let dir = probe_package_outside_a_workspace("features");
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::new(vec![FeatureName::new("recipe_no_such_feature".to_owned())]),
        Some(WorkingDir::new(dir.clone())),
        sample_channel(),
    );

    let tail = stderr_of_a_failed_launch(&spec);
    assert!(
        tail.contains("recipe_no_such_feature"),
        "cargo saw the recipe's features: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

#[test]
fn the_launch_port_reaches_the_real_child_on_the_channel_variable() {
    let channel = sample_channel();
    let port_var = channel.port().as_str().to_owned();
    let dir = probe_package_kept_warm("port", &probe_main_echoing(&port_var));
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir)),
        channel,
    );
    let port = free_port();

    // The first run compiles the probe; later runs find it built.
    let tail = stderr_of_a_failed_launch_on(McpPort::new(port), &spec);
    assert!(
        tail.contains(&format!("{PROBE_PORT_MARKER}={port}")),
        "the child read {port} back out of {port_var}: {tail}"
    );
}

#[test]
fn the_recipe_working_directory_is_where_cargo_runs() {
    let dir = temp_dir_outside_a_workspace("cwd");
    let spec = LaunchSpec::new(
        CargoPackage::new(SAMPLE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir.clone())),
        sample_channel(),
    );
    let mut manager =
        HostManager::with_config(Box::new(CargoSpawner::new()), no_boot_deadline_config());

    let outcome = manager.launch(McpPort::new(free_port()), &spec);
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
    let mut manager =
        HostManager::with_config(Box::new(CargoSpawner::new()), no_boot_deadline_config());

    for dir in [&first_dir, &second_dir] {
        let spec = LaunchSpec::new(
            CargoPackage::new(SAMPLE_PACKAGE.to_owned()),
            FeatureList::default(),
            Some(WorkingDir::new(dir.clone())),
            sample_channel(),
        );
        let outcome = manager.launch(McpPort::new(free_port()), &spec);
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

//! The launch recipe reaching the REAL launcher — the working directory, the features, and
//! the environment overrides (GTW-875).
//!
//! The stub spawner in [`support`](crate::support) runs a fixed `sh` placeholder, so it
//! can say nothing about whether a recipe's directory, features, or environment ever reach
//! a launcher. These tests use the production [`CargoSpawner`] and read cargo's own reply
//! back out of the child's captured stderr: a recipe pointed at a directory with no
//! `Cargo.toml` fails THERE, naming that directory; a recipe naming a feature the package
//! does not have fails naming that feature; a recipe setting `CARGO_BUILD_TARGET` to a
//! target triple that does not exist fails naming that triple. Each of those messages is
//! only reachable if the corresponding part of the recipe was applied to the real command.
//!
//! Nothing here builds the game: every launch dies in the time cargo takes to reject its
//! own arguments.

use std::{fs, path::PathBuf, process, time::SystemTime};

use gdtf_qa_mcp::{
    CargoPackage, CargoSpawner, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList,
    FeatureName, HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, LaunchSpec, QaChannel,
    QaPort, WorkingDir,
};

use crate::support::{fast_config, free_port};

/// The package name the throwaway manifest below declares.
const PROBE_PACKAGE: &str = "gtw875_probe";

/// Make a fresh directory under the system temp directory, outside any cargo workspace.
fn temp_dir_outside_a_workspace(tag: &str) -> PathBuf {
    let nanos = SystemTime::UNIX_EPOCH
        .elapsed()
        .map_or(0, |since| since.as_nanos());
    let path = std::env::temp_dir().join(format!("gtw875-{tag}-{}-{nanos}", process::id()));
    let Ok(()) = fs::create_dir_all(&path) else {
        unreachable!("the test can create a temp directory");
    };
    path
}

/// Make a directory holding the smallest possible cargo package, outside any workspace.
///
/// It exists so cargo gets PAST "no manifest here" and rejects the recipe's own parts
/// instead — a feature the package does not declare, or a target triple that does not
/// exist. It is never built: both rejections happen before any compilation.
fn probe_package_outside_a_workspace(tag: &str) -> PathBuf {
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
    let Ok(()) = fs::write(path.join("src/main.rs"), "fn main() {}\n") else {
        unreachable!("the test can write the probe main");
    };
    path
}

/// Launch `spec` through the REAL [`CargoSpawner`] and return the stderr of the child's
/// early exit.
fn stderr_of_a_failed_launch(spec: &LaunchSpec) -> String {
    let mut manager = HostManager::with_config(Box::new(CargoSpawner::new()), fast_config(30_000));
    let outcome = manager.launch(QaPort::new(free_port()), spec);
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("cargo rejects the recipe and exits at once: {outcome:?}");
    };
    tail.as_str().to_owned()
}

/// The recipe's FEATURES reach the real launcher: cargo rejects a feature the package does
/// not declare, and names it.
#[test]
fn the_recipe_features_reach_the_real_cargo_command() {
    let dir = probe_package_outside_a_workspace("features");
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::new(vec![FeatureName::new("gtw875_no_such_feature".to_owned())]),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        QaChannel::game(),
    );

    let tail = stderr_of_a_failed_launch(&spec);
    assert!(
        tail.contains("gtw875_no_such_feature"),
        "cargo saw the recipe's features: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

/// The recipe's ENVIRONMENT OVERRIDES reach the real child: cargo reads
/// `CARGO_BUILD_TARGET` from its own environment, and rejects a triple that does not
/// exist, naming it.
#[test]
fn the_recipe_environment_reaches_the_real_child() {
    let dir = probe_package_outside_a_workspace("env");
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::new(vec![EnvVar::new(
            EnvVarName::new("CARGO_BUILD_TARGET".to_owned()),
            EnvVarValue::new("gtw875-not-a-real-target".to_owned()),
        )]),
        QaChannel::game(),
    );

    let tail = stderr_of_a_failed_launch(&spec);
    assert!(
        tail.contains("gtw875-not-a-real-target"),
        "the child ran with the recipe's environment: {tail}"
    );
    drop(fs::remove_dir_all(&dir));
}

/// The recipe's working directory reaches the real launcher: cargo runs THERE, fails to
/// find a manifest, and names that very directory in the stderr the launch failure
/// carries.
#[test]
fn the_recipe_working_directory_is_where_cargo_runs() {
    let dir = temp_dir_outside_a_workspace("cwd");
    let spec = LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
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

/// Two launches in a row can name different recipes: the recipe belongs to the call, not
/// to the spawner, so one running host can drive one build and then another.
#[test]
fn successive_launches_can_name_different_recipes() {
    let first_dir = temp_dir_outside_a_workspace("first");
    let second_dir = temp_dir_outside_a_workspace("second");
    let mut manager = HostManager::with_config(Box::new(CargoSpawner::new()), fast_config(30_000));

    for dir in [&first_dir, &second_dir] {
        let spec = LaunchSpec::new(
            CargoPackage::new("grimdark_turfwar".to_owned()),
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

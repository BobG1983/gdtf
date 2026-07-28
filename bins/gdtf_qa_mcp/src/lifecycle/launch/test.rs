//! Unit tests for the launch recipe's values and the default game recipe (GTW-875).

use std::path::PathBuf;

use super::{
    spec::LaunchSpec,
    values::{EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName, WorkingDir},
};

/// A feature list renders as the comma-joined `--features` value; an empty one renders as
/// nothing at all, so the caller omits the flag rather than passing an empty string.
#[test]
fn feature_list_renders_comma_joined_and_empty_is_none() {
    let features = FeatureList::new(vec![
        FeatureName::new("dynamic_linking".to_owned()),
        FeatureName::new("net_qa".to_owned()),
        FeatureName::new("dev_tools".to_owned()),
    ]);
    assert_eq!(
        features.render(),
        Some("dynamic_linking,net_qa,dev_tools".to_owned())
    );
    assert_eq!(FeatureList::default().render(), None);
}

/// The default recipe is the pre-GTW-875 hardcoded launch: the game package, dynamic
/// linking plus the QA channel, no directory of its own, no extra environment.
#[test]
fn game_default_is_the_previous_hardcoded_recipe() {
    let spec = LaunchSpec::game_default();
    assert_eq!(spec.package().as_str(), "grimdark_turfwar");
    assert_eq!(
        spec.features().render(),
        Some("dynamic_linking,net_qa".to_owned())
    );
    assert!(spec.working_dir().is_none());
    assert!(spec.env().is_empty());
}

/// A recipe that names no directory resolves to the host's own, so two recipes that name
/// the same checkout two ways are ONE recipe — and any other difference is not.
#[test]
fn recipes_match_on_the_resolved_directory_and_differ_on_anything_else() {
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    let unnamed = LaunchSpec::game_default();
    let named = LaunchSpec::new(
        unnamed.package().clone(),
        unnamed.features().clone(),
        Some(WorkingDir::new(here)),
        EnvOverrides::default(),
    );
    assert_eq!(unnamed.resolved_working_dir(), named.resolved_working_dir());
    assert!(unnamed.is_same_launch_as(&named));
    assert!(named.is_same_launch_as(&unnamed));

    let elsewhere = LaunchSpec::new(
        unnamed.package().clone(),
        unnamed.features().clone(),
        Some(WorkingDir::new(PathBuf::from("/tmp/another-worktree"))),
        EnvOverrides::default(),
    );
    assert!(!unnamed.is_same_launch_as(&elsewhere), "a different tree");

    let with_dev_tools = LaunchSpec::new(
        unnamed.package().clone(),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        None,
        EnvOverrides::default(),
    );
    assert!(
        !unnamed.is_same_launch_as(&with_dev_tools),
        "different features"
    );

    let with_env = LaunchSpec::new(
        unnamed.package().clone(),
        unnamed.features().clone(),
        None,
        EnvOverrides::new(vec![EnvVar::new(
            EnvVarName::new("GDTF_BATTLE_SEED".to_owned()),
            EnvVarValue::new("42".to_owned()),
        )]),
    );
    assert!(
        !unnamed.is_same_launch_as(&with_env),
        "different environment"
    );
}

/// A recipe keeps the working directory and environment pairs it was built with, in order.
#[test]
fn spec_keeps_working_dir_and_env_overrides() {
    let spec = LaunchSpec::new(
        LaunchSpec::game_default().package().clone(),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        Some(WorkingDir::new(PathBuf::from("/tmp/some-worktree"))),
        EnvOverrides::new(vec![EnvVar::new(
            EnvVarName::new("GDTF_BATTLE_SEED".to_owned()),
            EnvVarValue::new("42".to_owned()),
        )]),
    );
    let Some(dir) = spec.working_dir() else {
        unreachable!("the recipe was built with a working directory");
    };
    assert_eq!(dir.to_str(), Some("/tmp/some-worktree"));
    let Some(first) = spec.env().first() else {
        unreachable!("the recipe was built with one environment override");
    };
    assert_eq!(first.name().as_str(), "GDTF_BATTLE_SEED");
    assert_eq!(first.value().as_str(), "42");
}

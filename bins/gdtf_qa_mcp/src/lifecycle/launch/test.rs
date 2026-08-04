use std::path::PathBuf;

use super::{
    channel::QaChannel,
    spec::LaunchSpec,
    values::{EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName, WorkingDir},
};

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
        QaChannel::game(),
    );
    assert_eq!(unnamed.resolved_working_dir(), named.resolved_working_dir());
    assert!(unnamed.is_same_launch_as(&named));
    assert!(named.is_same_launch_as(&unnamed));

    let elsewhere = LaunchSpec::new(
        unnamed.package().clone(),
        unnamed.features().clone(),
        Some(WorkingDir::new(PathBuf::from("/tmp/another-worktree"))),
        EnvOverrides::default(),
        QaChannel::game(),
    );
    assert!(!unnamed.is_same_launch_as(&elsewhere), "a different tree");

    let with_dev_tools = LaunchSpec::new(
        unnamed.package().clone(),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        None,
        EnvOverrides::default(),
        QaChannel::game(),
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
        QaChannel::game(),
    );
    assert!(
        !unnamed.is_same_launch_as(&with_env),
        "different environment"
    );

    let other_channel = LaunchSpec::new(
        unnamed.package().clone(),
        unnamed.features().clone(),
        None,
        EnvOverrides::default(),
        QaChannel::editor(),
    );
    assert!(
        !unnamed.is_same_launch_as(&other_channel),
        "different QA channel"
    );
}

#[test]
fn editor_default_is_the_editor_binary_over_the_editor_channel() {
    let spec = LaunchSpec::editor_default();
    assert_eq!(spec.package().as_str(), "gdtf_content_editor_bin");
    assert_eq!(
        spec.features().render(),
        Some("dynamic_linking,file_watcher,net_qa".to_owned())
    );
    assert_eq!(spec.channel().enable().as_str(), "GDTF_EDITOR_NET_QA");
    assert_eq!(spec.channel().port().as_str(), "GDTF_EDITOR_NET_QA_PORT");
    assert!(spec.working_dir().is_none());
    assert!(spec.env().is_empty());
    assert!(
        !spec.is_same_launch_as(&LaunchSpec::game_default()),
        "the two default recipes are different launches"
    );
}

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
        QaChannel::game(),
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

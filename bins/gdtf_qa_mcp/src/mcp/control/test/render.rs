use serde_json::json;

use super::support::a_directory_that_is_not_the_hosts;
use crate::{
    hosts::QaHost,
    lifecycle::{
        BootTimeout, CargoPackage, ChildPid, EnvOverrides, FailureTail, FeatureList, FeatureName,
        LaunchFailure, LaunchOutcome, LaunchSpec, OrphanPid, QaChannel, StopOutcome, WorkingDir,
    },
    link::QaPort,
    mcp::control::render::{render_launch, render_stop},
};

#[test]
fn already_running_reports_the_running_childs_recipe() {
    let dir = a_directory_that_is_not_the_hosts();
    let running = LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        QaChannel::game(),
    );
    let rendered = render_launch(
        QaHost::Game,
        &LaunchOutcome::AlreadyRunning {
            port:   QaPort::new(4321),
            pid:    ChildPid::new(999),
            recipe: Box::new(running),
        },
        &LaunchSpec::game_default(),
    );
    assert_eq!(rendered["isError"], json!(false));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    let Some(shown) = dir.to_str() else {
        unreachable!("the fixture directory is valid UTF-8");
    };
    assert!(text.contains("already_running"), "rendered: {text}");
    assert!(text.contains("dev_tools"), "rendered: {text}");
    assert!(text.contains(shown), "rendered: {text}");
}

#[test]
fn a_recipe_mismatch_is_a_tool_error_naming_both_recipes() {
    let dir = a_directory_that_is_not_the_hosts();
    let running = LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        FeatureList::new(vec![FeatureName::new("net_qa".to_owned())]),
        Some(WorkingDir::new(dir.clone())),
        EnvOverrides::default(),
        QaChannel::game(),
    );
    let requested = LaunchSpec::new(
        CargoPackage::new("gdtf_content_editor".to_owned()),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        None,
        EnvOverrides::default(),
        QaChannel::game(),
    );
    let rendered = render_launch(
        QaHost::Game,
        &LaunchOutcome::Failed(LaunchFailure::RecipeMismatch(Box::new(running))),
        &requested,
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    let Some(shown) = dir.to_str() else {
        unreachable!("the fixture directory is valid UTF-8");
    };
    assert!(text.contains(shown), "names the running tree: {text}");
    assert!(
        text.contains("gdtf_content_editor"),
        "names what was asked for: {text}"
    );
    assert!(
        text.contains("stop(host=\"game\")"),
        "says how to proceed: {text}"
    );
}

#[test]
fn render_launch_timeout_names_the_build_and_carries_the_tail() {
    let rendered = render_launch(
        QaHost::Editor,
        &LaunchOutcome::Failed(LaunchFailure::Timeout {
            tail:   FailureTail::new("Compiling gdtf_content_editor".to_owned()),
            waited: BootTimeout::new(core::time::Duration::from_secs(600)),
        }),
        &LaunchSpec::editor_default(),
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(
        text.contains("Compiling gdtf_content_editor"),
        "rendered: {text}"
    );
    assert!(
        text.contains("BUILD"),
        "names the build as the cause: {text}"
    );
    assert!(
        text.contains("cargo build -p gdtf_content_editor_bin --features dynamic_linking,file_watcher,net_qa"),
        "gives the warm-up command: {text}"
    );
    assert!(text.contains("600s"), "says how long it waited: {text}");
    assert!(text.contains("editor"), "names the host: {text}");
}

#[test]
fn a_named_orphan_that_survives_is_a_tool_error_naming_the_process() {
    let rendered = render_stop(
        QaHost::Editor,
        &StopOutcome::OrphanHeld {
            port: QaPort::new(7617),
            pid:  OrphanPid::Known(ChildPid::new(43744)),
        },
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains("editor"), "names the host: {text}");
    assert!(text.contains("7617"), "names the port: {text}");
    assert!(text.contains("43744"), "names the process: {text}");
    assert!(text.contains("orphan"), "says what it is: {text}");
    assert!(!text.contains("not_running"), "rendered: {text}");
}

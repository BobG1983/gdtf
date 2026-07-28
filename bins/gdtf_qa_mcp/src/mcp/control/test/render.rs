//! What each launch / stop outcome renders — including the recipe-mismatch error and the
//! timeout message that names the build as the cause (GTW-808 clause 7).

use serde_json::json;

use super::support::a_directory_that_is_not_the_hosts;
use crate::{
    hosts::QaHost,
    lifecycle::{
        BootTimeout, CargoPackage, ChildPid, EnvOverrides, FeatureList, FeatureName, LaunchFailure,
        LaunchOutcome, LaunchSpec, QaChannel, StderrTail, WorkingDir,
    },
    link::QaPort,
    mcp::control::render::render_launch,
};

/// An `already_running` answer reports the RUNNING child's recipe — the package,
/// features, and directory of the build the caller is about to drive — not the recipe this
/// call happened to ask for.
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

/// A recipe mismatch renders as a tool ERROR naming both the running recipe and the
/// requested one — never a success-shaped block for a build that was never started.
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
    assert!(text.contains("stop_game"), "says how to proceed: {text}");
}

/// A boot-timeout failure renders as a tool error carrying the stderr tail — and NAMES
/// THE BUILD as the likely cause, with the warm-up command, rather than reporting a bare
/// "timed out" (GTW-808 clause 7).
#[test]
fn render_launch_timeout_names_the_build_and_carries_the_tail() {
    let rendered = render_launch(
        QaHost::Editor,
        &LaunchOutcome::Failed(LaunchFailure::Timeout {
            tail:   StderrTail::new("Compiling gdtf_content_editor".to_owned()),
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
        text.contains("cargo build -p gdtf_content_editor_bin --features dynamic_linking,net_qa"),
        "gives the warm-up command: {text}"
    );
    assert!(text.contains("600s"), "says how long it waited: {text}");
    assert!(text.contains("editor"), "names the host: {text}");
}

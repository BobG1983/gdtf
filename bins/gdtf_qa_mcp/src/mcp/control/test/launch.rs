//! What a launch call reads — the port, the recipe, the host's defaults — and what it
//! hands the lifecycle.

use serde_json::json;

use super::support::{RecordingLink, StubLifecycle, a_directory_that_is_not_the_hosts};
use crate::{
    hosts::QaHost,
    lifecycle::{ChildPid, LaunchFailure, LaunchOutcome, SpawnError, StopOutcome},
    link::QaPort,
    mcp::{
        call::ToolCallOutcome,
        control::{
            handle::{handle_launch, handle_stop, parse_port},
            render::render_stop,
        },
    },
};

/// An explicit `port` argument parses to that port; a too-large one is rejected.
#[test]
fn parse_port_reads_explicit_and_rejects_out_of_range() {
    let Ok(port) = parse_port(QaHost::Game, &json!({ "port": 4321 })) else {
        unreachable!("an in-range port parses");
    };
    assert_eq!(*port, 4321);
    assert!(parse_port(QaHost::Game, &json!({ "port": 99999 })).is_err());
    assert!(parse_port(QaHost::Game, &json!({ "port": "nope" })).is_err());
}

/// A successful launch re-points the game link at the launched port and renders a
/// non-error block.
#[test]
fn handle_launch_retargets_link_and_renders_launched() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: QaPort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let ToolCallOutcome::Result(result) =
        handle_launch(QaHost::Game, &json!({}), &mut link, &mut lifecycle)
    else {
        unreachable!("a launch renders a result block");
    };
    assert_eq!(link.retargeted, Some(QaPort::new(4321)));
    assert_eq!(result["isError"], json!(false));
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    assert!(text.contains("launched"), "rendered: {text}");
    assert!(text.contains("4321"), "rendered: {text}");
}

/// The recipe arguments reach the lifecycle, and the rendered result reports the package,
/// features, and directory the launch actually ran with.
#[test]
fn handle_launch_passes_the_recipe_through_and_reports_it() {
    let dir = a_directory_that_is_not_the_hosts();
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: QaPort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let args = json!({
        "features": ["dynamic_linking", "net_qa", "dev_tools"],
        "working_dir": dir.to_string_lossy(),
        "env": { "GDTF_BATTLE_SEED": "42" },
    });
    let ToolCallOutcome::Result(result) =
        handle_launch(QaHost::Game, &args, &mut link, &mut lifecycle)
    else {
        unreachable!("a launch renders a result block");
    };
    let Some(spec) = lifecycle.last_spec.as_ref() else {
        unreachable!("the launch handed the lifecycle a recipe");
    };
    assert_eq!(
        spec.features().render(),
        Some("dynamic_linking,net_qa,dev_tools".to_owned())
    );
    assert_eq!(
        spec.working_dir().map(|path| path.to_path_buf()),
        Some(dir.clone())
    );
    assert_eq!(spec.env().len(), 1);
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    let Some(shown) = dir.to_str() else {
        unreachable!("the fixture directory is valid UTF-8");
    };
    assert!(text.contains("dev_tools"), "rendered: {text}");
    assert!(text.contains("grimdark_turfwar"), "rendered: {text}");
    // The VALUE, not just the key: a caller reads which checkout is under test off this.
    assert!(text.contains(shown), "rendered: {text}");
}

/// A recipe argument of the wrong shape is an invalid-params rejection, not a silent
/// fallback to the default recipe.
#[test]
fn a_bad_recipe_argument_is_rejected_before_any_launch() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: QaPort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let args = json!({ "working_dir": "/no/such/tree/here" });
    let ToolCallOutcome::Invalid(message) =
        handle_launch(QaHost::Game, &args, &mut link, &mut lifecycle)
    else {
        unreachable!("a non-existent working directory is rejected");
    };
    assert!(message.contains("working_dir"), "message: {message}");
    assert!(
        lifecycle.last_spec.is_none(),
        "nothing was launched: {:?}",
        lifecycle.last_spec
    );
}

/// A bare `launch_editor` hands the lifecycle the EDITOR's recipe, and the reply names it
/// — the two launch tools are not one tool with a flag (GTW-808 clause 1).
#[test]
fn handle_launch_for_the_editor_uses_the_editor_recipe() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: QaPort::new(7617),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let ToolCallOutcome::Result(result) =
        handle_launch(QaHost::Editor, &json!({}), &mut link, &mut lifecycle)
    else {
        unreachable!("a launch renders a result block");
    };
    let Some(spec) = lifecycle.last_spec.as_ref() else {
        unreachable!("the launch handed the lifecycle a recipe");
    };
    assert_eq!(spec.package().as_str(), "gdtf_content_editor_bin");
    assert_eq!(spec.channel().enable().as_str(), "GDTF_EDITOR_NET_QA");
    assert_eq!(link.retargeted, Some(QaPort::new(7617)));
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    assert!(text.contains("gdtf_content_editor_bin"), "rendered: {text}");
    assert!(text.contains("7617"), "rendered: {text}");
}

/// Stopping with nothing running renders a typed non-error `not_running` result.
#[test]
fn handle_stop_not_running_is_typed_result() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Failed(LaunchFailure::Spawn(
        SpawnError::new("unused".to_owned()),
    )));
    let ToolCallOutcome::Result(result) = handle_stop(&mut lifecycle) else {
        unreachable!("a stop renders a result block");
    };
    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        render_stop(&StopOutcome::NotRunning)["isError"],
        json!(false)
    );
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the stop reply is text content");
    };
    assert!(text.contains("not_running"), "rendered: {text}");
}

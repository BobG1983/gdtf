//! What a launch call reads — the port, the recipe, the host's defaults — and what it
//! hands the lifecycle.

use serde_json::json;

use super::support::{RecordingLink, StubLifecycle, a_directory_that_is_not_the_hosts};
use crate::{
    hosts::QaHost,
    lifecycle::{ChildPid, LaunchFailure, LaunchOutcome, OrphanPid, SpawnError, StopOutcome},
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

/// Stopping with nothing running renders a typed non-error `not_running` result — and the
/// handler hands the lifecycle the host's PORT, so that answer rests on an established
/// fact rather than on the host owning no handle (GTW-926).
#[test]
fn handle_stop_not_running_is_typed_result() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Failed(LaunchFailure::Spawn(
        SpawnError::new("unused".to_owned()),
    )));
    let ToolCallOutcome::Result(result) = handle_stop(QaHost::Game, &mut lifecycle) else {
        unreachable!("a stop renders a result block");
    };
    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        render_stop(QaHost::Game, &StopOutcome::NotRunning)["isError"],
        json!(false)
    );
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the stop reply is text content");
    };
    assert!(text.contains("not_running"), "rendered: {text}");
    assert_eq!(
        lifecycle.stopped_port,
        Some(QaHost::Game.port_from_env()),
        "the stop was told which port to establish the state of"
    );
}

/// EACH stop tool is handed ITS OWN host's port (GTW-926 clause 4).
///
/// The two hosts listen on different ports, so a handler that probed one fixed port would
/// leave the other host's orphan invisible — and the reported defect was on the EDITOR's
/// port, `stop_editor` answering `not_running` about a live child on 7617.
#[test]
fn each_stop_tool_is_handed_its_own_hosts_port() {
    assert_ne!(
        QaHost::Game.port_from_env(),
        QaHost::Editor.port_from_env(),
        "the two hosts listen on different ports, so the port a stop probes is host-specific"
    );
    for host in QaHost::ALL {
        let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Failed(LaunchFailure::Spawn(
            SpawnError::new("unused".to_owned()),
        )));
        let ToolCallOutcome::Result(_) = handle_stop(host, &mut lifecycle) else {
            unreachable!("a stop renders a result block");
        };
        assert_eq!(
            lifecycle.stopped_port,
            Some(host.port_from_env()),
            "{}: the stop establishes the state of that host's own port",
            host.label()
        );
    }
}

/// A stop that adopted and stopped an ORPHAN reports it as an orphan, with the port and the
/// process — never as `not_running` (GTW-926).
#[test]
fn a_stopped_orphan_is_reported_as_an_orphan() {
    let rendered = render_stop(
        QaHost::Editor,
        &StopOutcome::OrphanStopped {
            port: QaPort::new(7617),
            pid:  OrphanPid::Known(ChildPid::new(43744)),
        },
    );
    assert_eq!(rendered["isError"], json!(false));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("the stop reply is text content");
    };
    assert!(text.contains("orphan_stopped"), "rendered: {text}");
    assert!(text.contains("7617"), "rendered: {text}");
    assert!(text.contains("43744"), "rendered: {text}");
    assert!(!text.contains("not_running"), "rendered: {text}");
}

/// An orphan that could NOT be stopped is a tool error naming the port and the host —
/// the caller's next step is not the same as after a clean stop (GTW-926).
#[test]
fn an_orphan_that_survives_is_a_tool_error() {
    let rendered = render_stop(
        QaHost::Editor,
        &StopOutcome::OrphanHeld {
            port: QaPort::new(7617),
            pid:  OrphanPid::Unknown,
        },
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("the stop reply is text content");
    };
    assert!(text.contains("editor"), "rendered: {text}");
    assert!(text.contains("7617"), "rendered: {text}");
    assert!(!text.contains("not_running"), "rendered: {text}");
}

/// A launch into a port an orphan already holds renders the orphan — the port, the process,
/// and the stop tool to call — instead of a success (GTW-926). Asserted for the EDITOR
/// host, whose `stop_editor` the message must name.
#[test]
fn a_launch_into_a_held_port_reports_the_orphan() {
    let mut lifecycle =
        StubLifecycle::launching(LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan {
            port: QaPort::new(7617),
            pid:  OrphanPid::Known(ChildPid::new(43744)),
        }));
    let mut link = RecordingLink { retargeted: None };
    let ToolCallOutcome::Result(result) =
        handle_launch(QaHost::Editor, &json!({}), &mut link, &mut lifecycle)
    else {
        unreachable!("a launch renders a result block");
    };
    assert_eq!(result["isError"], json!(true));
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    assert!(text.contains("7617"), "rendered: {text}");
    assert!(text.contains("43744"), "rendered: {text}");
    assert!(text.contains("stop_editor"), "rendered: {text}");
    assert!(text.contains("orphan"), "rendered: {text}");
    assert!(!text.contains("\"status\""), "rendered: {text}");
    assert_eq!(
        link.retargeted, None,
        "a failed launch re-points nothing at the orphan's port"
    );
}

//! Unit tests for the two host-local tools: the recipe and port they read, the link
//! re-pointing, and what each outcome renders.

use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};
use serde_json::json;

use super::{
    handle::{handle_launch, handle_stop, parse_port},
    render::{render_launch, render_stop},
};
use crate::{
    error::McpError,
    game::{GameLink, GamePort},
    lifecycle::{
        CargoPackage, ChildPid, EnvOverrides, FeatureList, FeatureName, GameLifecycle,
        LaunchFailure, LaunchOutcome, LaunchSpec, SpawnError, StderrTail, StopOutcome, WorkingDir,
    },
    mcp::call::ToolCallOutcome,
};

/// A lifecycle whose launch / stop return canned outcomes — no real processes. It keeps
/// the recipe it was handed, so a test can check what the tool asked for.
struct StubLifecycle {
    /// The outcome `launch` returns.
    launch:    LaunchOutcome,
    /// The outcome `stop` returns.
    stop:      StopOutcome,
    /// The recipe of the last `launch` call, if any.
    last_spec: Option<LaunchSpec>,
}

impl StubLifecycle {
    /// A stub that reports the given launch outcome and nothing to stop.
    const fn launching(launch: LaunchOutcome) -> Self {
        Self {
            launch,
            stop: StopOutcome::NotRunning,
            last_spec: None,
        }
    }
}

impl GameLifecycle for StubLifecycle {
    fn launch(&mut self, _port: GamePort, spec: &LaunchSpec) -> LaunchOutcome {
        self.last_spec = Some(spec.clone());
        self.launch.clone()
    }

    fn stop(&mut self) -> StopOutcome {
        self.stop.clone()
    }
}

/// A game link that records the last port it was re-pointed at.
struct RecordingLink {
    /// The port the last `retarget` set, if any.
    retargeted: Option<GamePort>,
}

impl GameLink for RecordingLink {
    fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
        Err(McpError::Disconnected)
    }

    fn retarget(&mut self, port: GamePort) {
        self.retargeted = Some(port);
    }
}

/// An explicit `port` argument parses to that port; a too-large one is rejected.
#[test]
fn parse_port_reads_explicit_and_rejects_out_of_range() {
    let Ok(port) = parse_port(&json!({ "port": 4321 })) else {
        unreachable!("an in-range port parses");
    };
    assert_eq!(*port, 4321);
    assert!(parse_port(&json!({ "port": 99999 })).is_err());
    assert!(parse_port(&json!({ "port": "nope" })).is_err());
}

/// A successful launch re-points the game link at the launched port and renders a
/// non-error block.
#[test]
fn handle_launch_retargets_link_and_renders_launched() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: GamePort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let ToolCallOutcome::Result(result) = handle_launch(&json!({}), &mut link, &mut lifecycle)
    else {
        unreachable!("a launch renders a result block");
    };
    assert_eq!(link.retargeted, Some(GamePort::new(4321)));
    assert_eq!(result["isError"], json!(false));
    let Some(text) = result["content"][0]["text"].as_str() else {
        unreachable!("the launch reply is text content");
    };
    assert!(text.contains("launched"), "rendered: {text}");
    assert!(text.contains("4321"), "rendered: {text}");
}

/// A real directory that is NOT the test process's own.
///
/// The recipe's directory falls back to the host's current directory when the recipe names
/// none, so a fixture equal to that current directory would let a launch that dropped the
/// recipe's directory entirely still render the expected value. This one cannot.
fn a_directory_that_is_not_the_hosts() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    assert!(dir.is_dir(), "the system temp directory exists: {dir:?}");
    assert_ne!(dir, here, "the fixture directory differs from the host's");
    dir
}

/// The recipe arguments reach the lifecycle, and the rendered result reports the package,
/// features, and directory the launch actually ran with.
#[test]
fn handle_launch_passes_the_recipe_through_and_reports_it() {
    let dir = a_directory_that_is_not_the_hosts();
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: GamePort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let args = json!({
        "features": ["dynamic_linking", "net_qa", "dev_tools"],
        "working_dir": dir.to_string_lossy(),
        "env": { "GDTF_BATTLE_SEED": "42" },
    });
    let ToolCallOutcome::Result(result) = handle_launch(&args, &mut link, &mut lifecycle) else {
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
    );
    let rendered = render_launch(
        &LaunchOutcome::AlreadyRunning {
            port:   GamePort::new(4321),
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
    );
    let requested = LaunchSpec::new(
        CargoPackage::new("gdtf_content_editor".to_owned()),
        FeatureList::new(vec![FeatureName::new("dev_tools".to_owned())]),
        None,
        EnvOverrides::default(),
    );
    let rendered = render_launch(
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

/// A recipe argument of the wrong shape is an invalid-params rejection, not a silent
/// fallback to the default recipe.
#[test]
fn a_bad_recipe_argument_is_rejected_before_any_launch() {
    let mut lifecycle = StubLifecycle::launching(LaunchOutcome::Launched {
        port: GamePort::new(4321),
        pid:  ChildPid::new(999),
    });
    let mut link = RecordingLink { retargeted: None };
    let args = json!({ "working_dir": "/no/such/tree/here" });
    let ToolCallOutcome::Invalid(message) = handle_launch(&args, &mut link, &mut lifecycle) else {
        unreachable!("a non-existent working directory is rejected");
    };
    assert!(message.contains("working_dir"), "message: {message}");
    assert!(
        lifecycle.last_spec.is_none(),
        "nothing was launched: {:?}",
        lifecycle.last_spec
    );
}

/// A boot-timeout failure renders as a tool error carrying the stderr tail.
#[test]
fn render_launch_timeout_is_tool_error_with_tail() {
    let rendered = render_launch(
        &LaunchOutcome::Failed(LaunchFailure::Timeout(StderrTail::new(
            "panicked at boot".to_owned(),
        ))),
        &LaunchSpec::game_default(),
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains("panicked at boot"), "rendered: {text}");
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

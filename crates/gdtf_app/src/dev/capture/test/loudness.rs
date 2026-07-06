//! GTW-590 C4c EMISSION pins — the loud lines are not merely classified and rendered
//! (the sibling `resolve` tests) but actually EMITTED on the real paths, each observed
//! under the GTW-455 [`capture_logs`] global log capture: the `from_env` warn loop
//! (its factored [`warn_config_diagnostics`] choke point), the real `Plugin::build`'s
//! `error!` for an uncreatable output directory, and a trigger's skip `warn!`.
//! Deleting any of those emit calls turns these red instead of turning a QA run
//! silent — the exact failure class GTW-590 exists to kill.
//!
//! Every emission is driven SYNCHRONOUSLY on the test thread (a plain fn call, the
//! synchronous `add_plugins` → `build`, `run_system_once`) so the thread-local
//! capture buffer records it — the [`capture_logs`] contract.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::acts::FireRequested;

use super::super::{
    diagnostics::warn_config_diagnostics,
    plugin::DevCapturePlugin,
    resolve::{RawCaptureEnv, resolve},
};
use crate::{
    dev::drive::{
        trigger_config::{FireAtFrame, FireConfig},
        triggers::trigger_fire_at_frame,
    },
    states::hot_reload_test_support::capture_logs,
};

/// C4c — [`warn_config_diagnostics`] (the sole emission choke point `from_env` runs)
/// EMITS one `dev-capture: `-prefixed `warn!` line per diagnostic, driven with an
/// injected warning-bearing snapshot through the REAL [`resolve`] path.
///
/// PIN: goes red if the warn loop is deleted or stops rendering the diagnostics —
/// every misconfiguration would go silent again.
#[test]
fn config_diagnostics_emit_their_loud_warn_lines() {
    // A blank path AND an orphan mode override: two INDEPENDENT diagnostics, so the
    // one-line-per-warning shape is pinned, not just "something was logged".
    let resolved = resolve(&RawCaptureEnv {
        path: Some("   ".to_owned()),
        fire_mode: Some("full".to_owned()),
        ..RawCaptureEnv::default()
    });
    assert_eq!(
        resolved.warnings.len(),
        2,
        "the snapshot raises exactly two diagnostics: {:?}",
        resolved.warnings,
    );

    let captured = capture_logs(|| warn_config_diagnostics(&resolved));

    assert!(
        captured.iter().any(|line| line
            .contains("dev-capture: GDTF_CAPTURE_PATH is set but blank; capture stays OFF")),
        "the blank-path diagnostic must be EMITTED as its loud warn!; captured: {captured:?}",
    );
    assert!(
        captured.iter().any(|line| line.contains(
            "dev-capture: GDTF_FIRE_MODE is set but GDTF_FIRE_AT_FRAME is not; the mode \
             override does nothing"
        )),
        "the orphan-mode diagnostic must be EMITTED as its loud warn!; captured: {captured:?}",
    );
}

/// C4c — the REAL [`DevCapturePlugin`] `build()` EMITS its loud `error!` (naming the
/// output path) when the capture output directory cannot be created — a parent
/// blocked by a plain file. The sibling `schedule` test pins the `Err`
/// CLASSIFICATION; this pins the EMISSION through `build()` itself (`add_plugins`
/// runs it synchronously on this thread).
///
/// PIN: goes red if `build()` stops logging the `ensure_output_dir` failure — a
/// misconfigured QA run would again produce zero PNGs with zero evidence.
#[test]
fn blocked_output_dir_emits_the_loud_error_line() {
    let Ok(dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };
    let plain_file = dir.path().join("plain_file");
    assert!(std::fs::write(&plain_file, b"not a directory").is_ok());
    let raw = RawCaptureEnv {
        path: Some(plain_file.join("shot.png").to_string_lossy().into_owned()),
        ..RawCaptureEnv::default()
    };
    let resolved = resolve(&raw);

    let captured = capture_logs(|| {
        let mut app = App::new();
        // The REAL plugin, built exactly as `from_env` builds it: `add_plugins` runs
        // `build()` synchronously, so its `error!` lands in this thread's capture.
        app.add_plugins(DevCapturePlugin::from_resolved(resolved));
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("dev-capture: cannot create the capture output directory")
                && line.contains("plain_file")
        }),
        "build() must error! the uncreatable output directory, naming the path; \
         captured: {captured:?}",
    );
}

/// C4c — a trigger SKIP is loud: on its one trigger frame with NO shooter selected,
/// the REAL [`trigger_fire_at_frame`] system EMITS its skip `warn!` (naming the frame
/// and the cause) and writes nothing on the fire path.
///
/// PIN: goes red if the skip branch goes back to a silent no-op — the dead-QA-run
/// class where the scripted shot simply never happens.
#[test]
fn fire_trigger_skip_emits_the_loud_warn_line() {
    let mut app = App::new();
    app.add_message::<FireRequested>();
    // Trigger frame 1 with the EMPTY selection: the one frame the scripted shot could
    // fire, so the skip must warn.
    app.insert_resource(FireConfig::new(FireAtFrame::new(1), None));
    app.insert_resource(SelectedShooter::cleared());
    app.insert_resource(SelectedFireMode::default());

    let captured = capture_logs(|| {
        // `run_system_once` drives the REAL system synchronously on this thread (the
        // capture_logs contract); its `Local` frame counter starts at 0, so this run
        // IS the configured trigger frame 1.
        let result = app.world_mut().run_system_once(trigger_fire_at_frame);
        assert!(result.is_ok(), "the trigger system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("dev-capture: fire trigger at frame 1")
                && line.contains("no shooter selected")
                && line.contains("skipped")
        }),
        "the no-shooter skip must emit its loud warn!; captured: {captured:?}",
    );
    // And the skip really skipped: nothing was written on the real fire path.
    assert!(
        app.world().resource::<Messages<FireRequested>>().is_empty(),
        "a skipped trigger writes no FireRequested",
    );
}

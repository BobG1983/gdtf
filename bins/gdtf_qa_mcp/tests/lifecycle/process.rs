//! The launch / stop paths against a REAL placeholder child process (GTW-745).
//!
//! Every step the manager takes here is production code driving a real process: a real
//! spawn into its own process group, a real stderr pipe drained by the real reader thread,
//! a real SIGTERM→SIGKILL→reap stop. Only the readiness endpoint (a fake listener) and the
//! program that gets launched (a `sh` placeholder instead of the game) come from
//! [`support`](crate::support).

use gdtf_qa_mcp::{
    GameLifecycle, GameManager, GamePort, LaunchFailure, LaunchOutcome, StopOutcome,
};

use crate::support::{STUB_STDERR_LINE, StubSpawner, fast_config, free_port, spawn_fake_game};

/// A launch against a listening fake server becomes ready; a stop then reaps the child and
/// a second stop reports nothing running.
#[test]
fn launch_becomes_ready_then_stops() {
    let port = spawn_fake_game();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));

    let outcome = manager.launch(GamePort::new(port));
    let LaunchOutcome::Launched {
        port: ready_port,
        pid,
    } = outcome
    else {
        unreachable!("launch against the fake server becomes ready: {outcome:?}");
    };
    assert_eq!(*ready_port, port);

    let stopped = manager.stop();
    let StopOutcome::Stopped { pid: stopped_pid } = stopped else {
        unreachable!("stop reaps the running child: {stopped:?}");
    };
    assert_eq!(pid, stopped_pid, "stop reaps the child that launched");
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

/// A second launch while a child is already running is ensure-style — the same child, no
/// second spawn.
#[test]
fn second_launch_is_already_running() {
    let port = spawn_fake_game();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));

    let LaunchOutcome::Launched { pid: first_pid, .. } = manager.launch(GamePort::new(port)) else {
        unreachable!("the first launch becomes ready");
    };
    let second = manager.launch(GamePort::new(port));
    let LaunchOutcome::AlreadyRunning {
        port: existing_port,
        pid,
    } = second
    else {
        unreachable!("a second launch is already-running, not a second spawn: {second:?}");
    };
    assert_eq!(*existing_port, port);
    assert_eq!(pid, first_pid, "already-running reports the same child");

    let _ = manager.stop();
}

/// With no readiness endpoint the launch times out, kills the orphaned child, and returns
/// a typed failure carrying the child's captured stderr tail.
///
/// The tail is captured through the whole real chain — the child's write, the pipe, the
/// reader thread, the ring — and the spawner already waited for that capture, so this
/// assertion tests the chain rather than whether the child won a footrace with the boot
/// timeout (GTW-756). What the timeout still owns is that the tail SURVIVES the kill and
/// reap and reaches the caller.
#[test]
fn launch_times_out_and_captures_stderr() {
    let port = free_port();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(800));

    let outcome = manager.launch(GamePort::new(port));
    let LaunchOutcome::Failed(LaunchFailure::Timeout(tail)) = outcome else {
        unreachable!("no listener means the launch times out: {outcome:?}");
    };
    assert!(
        tail.contains(STUB_STDERR_LINE),
        "the failure carries the child's stderr tail: {}",
        tail.as_str()
    );
    // The orphaned child was already killed and reaped, so there is nothing left to stop.
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

/// Stopping with nothing running is a typed no-op, never a hang or a signal to a dead pid.
#[test]
fn stop_with_nothing_running_is_not_running() {
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

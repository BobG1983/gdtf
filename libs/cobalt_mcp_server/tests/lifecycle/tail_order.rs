use std::sync::{Arc, Mutex};

use cobalt_mcp_server::{
    HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, McpPort, StopOutcome,
    lifecycle::ChildStatus,
};

use crate::{
    fake_child::{CallLog, ChildCall, GATED_LINE, ReapGatedSpawner, recorded},
    support::{fast_config, free_port, sample_spec},
};

const NO_WAIT_BOOT_MS: u64 = 0;

const UNREACHED_BOOT_MS: u64 = 2000;

fn manager_over_gated_child(status: ChildStatus, boot_ms: u64) -> (HostManager, CallLog) {
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let spawner = ReapGatedSpawner::new(status, Arc::clone(&calls));
    let manager = HostManager::with_config(Box::new(spawner), fast_config(boot_ms));
    (manager, calls)
}

#[test]
fn timeout_reads_the_output_tail_after_reaping_the_orphan() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Running, NO_WAIT_BOOT_MS);

    let outcome = manager.launch(McpPort::new(free_port()), &sample_spec());
    let LaunchOutcome::Failed(LaunchFailure::Timeout { tail, .. }) = outcome else {
        unreachable!("no readiness endpoint means the launch times out: {outcome:?}");
    };
    assert_eq!(
        tail.as_str(),
        GATED_LINE,
        "the timeout failure carries the tail the child only yields once reaped",
    );
    assert_eq!(
        recorded(&calls),
        vec![
            ChildCall::Poll,
            ChildCall::Terminate,
            ChildCall::WaitUntilExit,
            ChildCall::Kill,
            ChildCall::Reap,
            ChildCall::ReadFailureTail,
        ],
        "the orphan is stopped and reaped, and only then is its output tail read",
    );
    assert_eq!(
        manager.stop(McpPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

#[test]
fn early_exit_reads_the_output_tail_after_reaping_the_child() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Exited, UNREACHED_BOOT_MS);

    let outcome = manager.launch(McpPort::new(free_port()), &sample_spec());
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("a child that has exited fails the launch as an early exit: {outcome:?}");
    };
    assert_eq!(
        tail.as_str(),
        GATED_LINE,
        "the early-exit failure carries the tail the child only yields once reaped",
    );
    assert_eq!(
        recorded(&calls),
        vec![ChildCall::Poll, ChildCall::Reap, ChildCall::ReadFailureTail],
        "the exited child is reaped, and only then is its output tail read",
    );
    assert_eq!(
        manager.stop(McpPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

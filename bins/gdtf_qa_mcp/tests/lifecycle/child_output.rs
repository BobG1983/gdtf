//! What the REAL [`HostManager`] answers when asked what its child has printed (GTW-943).
//!
//! This is the fact the `logs` tool reads. The manager is production code here — a real
//! spawn of a real process, a real readiness probe against the fake listener, a real stop —
//! so the tail comes from the child the manager actually owns rather than from a fixture
//! standing in for it. `output_tail.rs` covers the ring underneath; this covers the one
//! line that joins the tool to it.

use gdtf_qa_mcp::{
    HostLifecycle, HostManager, LaunchOutcome, LaunchSpec, QaPort, StopOutcome, TailLines,
};

use crate::support::{GatedStubSpawner, STUB_STDERR_LINE, fast_config, spawn_gated_fake_game};

/// With a child running, the manager hands back THAT child's captured output; with none, it
/// answers `None`.
///
/// `None` and an empty tail are different facts — the `logs` tool renders the first as "no
/// child is running" and the second as a silent process — so both ends are asserted. The
/// placeholder writes [`STUB_STDERR_LINE`] before it sleeps, and the spawner does not return
/// until that line has reached the ring, so the assertion races nothing.
#[test]
fn a_running_child_reports_what_it_printed() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));

    assert_eq!(
        manager.child_output(TailLines::default()),
        None,
        "with no child running there is no output to report",
    );

    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );

    let Some(tail) = manager.child_output(TailLines::default()) else {
        unreachable!("a running child has a tail to report");
    };
    assert!(
        tail.contains(STUB_STDERR_LINE),
        "the manager reports the RUNNING child's own output, got {tail:?}",
    );

    let stopped = manager.stop(QaPort::new(port));
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
    assert_eq!(
        manager.child_output(TailLines::default()),
        None,
        "a stopped child leaves no output behind",
    );
}

/// The line cap the caller asked for reaches the CHILD's ring, not just the reply.
///
/// A cap of zero is the discriminating case: the manager still owns a child, so the honest
/// answer is `Some` of an EMPTY tail — "I have a child and you asked for none of its lines".
/// A manager that dropped the cap on the floor would hand back the placeholder's line here.
#[test]
fn the_line_cap_reaches_the_childs_ring() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );

    let Some(capped) = manager.child_output(TailLines::new(0)) else {
        unreachable!("a running child answers Some even under a zero cap");
    };
    assert!(
        capped.is_empty(),
        "a cap of zero returns no lines from the running child, got {capped:?}",
    );

    let stopped = manager.stop(QaPort::new(port));
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
}

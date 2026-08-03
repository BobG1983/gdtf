use gdtf_qa_mcp::{
    HostLifecycle, HostManager, LaunchOutcome, LaunchSpec, QaPort, StopOutcome, TailLines,
};

use crate::support::{GatedStubSpawner, STUB_STDERR_LINE, fast_config, spawn_gated_fake_game};

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

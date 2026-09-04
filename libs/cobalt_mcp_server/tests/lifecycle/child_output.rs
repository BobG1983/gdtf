use std::sync::{Arc, Mutex};

use cobalt_mcp_server::{
    HostLifecycle, HostManager, LaunchOutcome, QaPort, StopOutcome, TailLines,
};

use crate::{
    fake_child::{CallLog, PortGatedSpawner, tail_on},
    support::{
        GatedStubSpawner, STUB_STDERR_LINE, WatchFreePort, always_spawning_config, fast_config,
        gated_listeners, sample_spec, spawn_gated_fake_game,
    },
};

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

    let outcome = manager.launch(QaPort::new(port), &sample_spec());
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
    let outcome = manager.launch(QaPort::new(port), &sample_spec());
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

#[test]
fn with_two_children_recorded_the_tail_is_the_last_one_launched() {
    let gates = gated_listeners(2);
    let ports: Vec<QaPort> = gates.iter().map(|(port, _)| *port).collect();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(PortGatedSpawner::new(calls, gates)),
        always_spawning_config(2000),
        Box::new(WatchFreePort),
    );
    let (Some(first), Some(second)) = (ports.first().copied(), ports.get(1).copied()) else {
        unreachable!("the test opened two gated listeners, got: {ports:?}");
    };

    let earlier = manager.launch(first, &sample_spec());
    let later = manager.launch(second, &sample_spec());

    assert!(
        matches!(earlier, LaunchOutcome::Launched { .. }),
        "the first launch becomes ready: {earlier:?}",
    );
    assert!(
        matches!(later, LaunchOutcome::Launched { .. }),
        "the second launch becomes ready: {later:?}",
    );
    let Some(tail) = manager.child_output(TailLines::default()) else {
        unreachable!("two recorded children leave a tail to report");
    };
    assert_eq!(
        *tail,
        tail_on(second),
        "with no instance named the tail is the last child launched, not the first",
    );
}

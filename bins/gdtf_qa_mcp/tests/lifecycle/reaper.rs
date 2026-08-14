use std::sync::{Arc, Mutex};

use gdtf_qa_mcp::{
    ChildLiveness, ChildPid, HostLifecycle, HostManager, LaunchOutcome, LaunchSpec, OrphanStop,
    OrphanTarget, OrphanWatch, PortHold, ProbeTimeout, QaPort, StopOutcome, SystemOrphanWatch,
    lifecycle::ChildStatus,
};

use crate::{
    fake_child::{CallLog, ChildCall, ReapGatedSpawner, recorded},
    support::{fast_config, spawn_gated_fake_game},
};

const BOOT_MS: u64 = 2000;

type ProbeLog = Arc<Mutex<Vec<ChildPid>>>;

struct FixedLiveness {
    answer: ChildStatus,
    asked:  ProbeLog,
}

impl ChildLiveness for FixedLiveness {
    fn status(&self, pid: ChildPid) -> ChildStatus {
        if let Ok(mut asked) = self.asked.lock() {
            asked.push(pid);
        }
        self.answer
    }
}

struct WatchFreePort;

impl OrphanWatch for WatchFreePort {
    fn inspect(&self, _port: QaPort, _timeout: ProbeTimeout) -> PortHold {
        PortHold::Free
    }

    fn stop(&self, _target: OrphanTarget) -> OrphanStop {
        OrphanStop::Stopped
    }
}

struct Fixture {
    manager: HostManager,
    calls:   CallLog,
    asked:   ProbeLog,
    port:    QaPort,
}

fn manager_over_a_gated_listener(answer: ChildStatus, orphans: Box<dyn OrphanWatch>) -> Fixture {
    let (port, gate) = spawn_gated_fake_game();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let asked: ProbeLog = Arc::new(Mutex::new(Vec::new()));
    let manager = HostManager::with_probes(
        Box::new(ReapGatedSpawner::gated(
            ChildStatus::Running,
            Arc::clone(&calls),
            gate,
        )),
        fast_config(BOOT_MS),
        orphans,
        Box::new(FixedLiveness {
            answer,
            asked: Arc::clone(&asked),
        }),
    );
    Fixture {
        manager,
        calls,
        asked,
        port: QaPort::new(port),
    }
}

fn launched_pid(fixture: &mut Fixture) -> ChildPid {
    let outcome = fixture
        .manager
        .launch(fixture.port, &LaunchSpec::game_default());
    let LaunchOutcome::Launched { pid, .. } = outcome else {
        unreachable!("the gated listener answers the readiness probe, got: {outcome:?}");
    };
    pid
}

fn asked_about(log: &ProbeLog) -> Vec<ChildPid> {
    log.lock().map(|asked| asked.clone()).unwrap_or_default()
}

#[test]
fn one_reaper_call_clears_a_record_whose_child_the_probe_calls_dead() {
    let mut fixture =
        manager_over_a_gated_listener(ChildStatus::Exited, Box::new(SystemOrphanWatch::new()));
    let launched = launched_pid(&mut fixture);

    fixture.manager.reap_dead_child();

    assert_eq!(
        asked_about(&fixture.asked),
        vec![launched],
        "the reaper asks the liveness probe about the recorded pid and nothing else"
    );
    assert_eq!(
        fixture.manager.stop_owned(),
        StopOutcome::NotRunning,
        "the dead child's record is gone, so there is nothing left to stop"
    );
    assert!(
        recorded(&fixture.calls).contains(&ChildCall::Reap),
        "the reaper reaped the dead child, calls: {:?}",
        recorded(&fixture.calls)
    );
}

#[test]
fn one_reaper_call_keeps_a_record_whose_child_the_probe_calls_alive() {
    let mut fixture =
        manager_over_a_gated_listener(ChildStatus::Running, Box::new(SystemOrphanWatch::new()));
    let launched = launched_pid(&mut fixture);

    fixture.manager.reap_dead_child();

    assert_eq!(
        fixture.manager.stop_owned(),
        StopOutcome::Stopped { pid: launched },
        "the probe answered alive, so the record survives the reaper"
    );
}

#[test]
fn a_reaper_call_with_no_record_touches_nothing() {
    let mut fixture =
        manager_over_a_gated_listener(ChildStatus::Exited, Box::new(SystemOrphanWatch::new()));

    fixture.manager.reap_dead_child();
    fixture.manager.reap_dead_child();

    assert!(
        asked_about(&fixture.asked).is_empty(),
        "with no record there is no pid to ask about, asked: {:?}",
        asked_about(&fixture.asked)
    );
    assert_eq!(fixture.manager.stop_owned(), StopOutcome::NotRunning);
    assert!(
        recorded(&fixture.calls).is_empty(),
        "no child was spawned, so none was touched, calls: {:?}",
        recorded(&fixture.calls)
    );
}

#[test]
fn launch_starts_a_new_child_once_the_reaper_has_cleared_the_record() {
    let mut fixture = manager_over_a_gated_listener(ChildStatus::Exited, Box::new(WatchFreePort));
    let first = launched_pid(&mut fixture);

    fixture.manager.reap_dead_child();

    let outcome = fixture
        .manager
        .launch(fixture.port, &LaunchSpec::game_default());
    let LaunchOutcome::Launched { pid, .. } = outcome else {
        unreachable!("the cleared record lets the launch spawn again, got: {outcome:?}");
    };
    assert_ne!(
        pid, first,
        "the relaunch reports the new child's pid, not the pid the cleared record held"
    );
}

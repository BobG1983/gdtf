use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mcp::{
    ChildLiveness, ChildPid, HostLifecycle, HostManager, InstanceId, LaunchOutcome, LaunchSpec,
    LifecycleConfig, OrphanWatch, QaPort, StopOutcome, SystemOrphanWatch, TailLines,
    lifecycle::ChildStatus,
};

use crate::{
    fake_child::{CallLog, ChildCall, PortGatedSpawner, counted, recorded, tail_on},
    support::{WatchFreePort, always_spawning_config, fast_config, gated_listeners},
};

const BOOT_MS: u64 = 2000;

type ProbeLog = Arc<Mutex<Vec<ChildPid>>>;

type PerPid = Arc<Mutex<HashMap<ChildPid, ChildStatus>>>;

#[derive(Clone)]
struct LivenessAnswer {
    default: ChildStatus,
    per_pid: PerPid,
}

impl LivenessAnswer {
    fn always(status: ChildStatus) -> Self {
        Self {
            default: status,
            per_pid: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn says(&self, pid: ChildPid, status: ChildStatus) {
        if let Ok(mut per_pid) = self.per_pid.lock() {
            per_pid.insert(pid, status);
        }
    }
}

struct FixedLiveness {
    answer: LivenessAnswer,
    asked:  ProbeLog,
}

impl ChildLiveness for FixedLiveness {
    fn status(&self, pid: ChildPid) -> ChildStatus {
        if let Ok(mut asked) = self.asked.lock() {
            asked.push(pid);
        }
        self.answer
            .per_pid
            .lock()
            .ok()
            .and_then(|per_pid| per_pid.get(&pid).copied())
            .unwrap_or(self.answer.default)
    }
}

struct Fixture {
    manager: HostManager,
    calls:   CallLog,
    asked:   ProbeLog,
    port:    QaPort,
    ports:   Vec<QaPort>,
}

fn manager_over_a_gated_listener(
    listeners: usize,
    answer: LivenessAnswer,
    orphans: Box<dyn OrphanWatch>,
    config: LifecycleConfig,
) -> Fixture {
    let gates = gated_listeners(listeners);
    let ports: Vec<QaPort> = gates.iter().map(|(port, _)| *port).collect();
    let Some(first) = ports.first().copied() else {
        unreachable!("the fixture opens at least one gated listener");
    };
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let asked: ProbeLog = Arc::new(Mutex::new(Vec::new()));
    let manager = HostManager::with_probes(
        Box::new(PortGatedSpawner::new(Arc::clone(&calls), gates)),
        config,
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
        port: first,
        ports,
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

fn launched_at(fixture: &mut Fixture, at: usize) -> (InstanceId, ChildPid) {
    let Some(port) = fixture.ports.get(at).copied() else {
        unreachable!(
            "the fixture opened a listener at {at}, ports: {:?}",
            fixture.ports
        );
    };
    let outcome = fixture.manager.launch(port, &LaunchSpec::game_default());
    let LaunchOutcome::Launched { instance, pid, .. } = outcome else {
        unreachable!("the launch at {at} becomes ready, got: {outcome:?}");
    };
    (instance, pid)
}

fn asked_about(log: &ProbeLog) -> Vec<ChildPid> {
    log.lock().map(|asked| asked.clone()).unwrap_or_default()
}

fn holds(fixture: &Fixture, id: &InstanceId) -> bool {
    fixture
        .manager
        .instances()
        .iter()
        .any(|instance| instance.id() == id)
}

#[test]
fn one_reaper_call_clears_a_record_whose_child_the_probe_calls_dead() {
    let mut fixture = manager_over_a_gated_listener(
        1,
        LivenessAnswer::always(ChildStatus::Exited),
        Box::new(SystemOrphanWatch::new()),
        fast_config(BOOT_MS),
    );
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
    let mut fixture = manager_over_a_gated_listener(
        1,
        LivenessAnswer::always(ChildStatus::Running),
        Box::new(SystemOrphanWatch::new()),
        fast_config(BOOT_MS),
    );
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
    let mut fixture = manager_over_a_gated_listener(
        1,
        LivenessAnswer::always(ChildStatus::Exited),
        Box::new(SystemOrphanWatch::new()),
        fast_config(BOOT_MS),
    );

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
    let mut fixture = manager_over_a_gated_listener(
        1,
        LivenessAnswer::always(ChildStatus::Exited),
        Box::new(WatchFreePort),
        fast_config(BOOT_MS),
    );
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

#[test]
fn one_reaper_call_drops_the_dead_instance_and_keeps_the_live_one() {
    for dead_at in 0..2 {
        let answer = LivenessAnswer::always(ChildStatus::Running);
        let mut fixture = manager_over_a_gated_listener(
            2,
            answer.clone(),
            Box::new(WatchFreePort),
            always_spawning_config(BOOT_MS),
        );
        let first = launched_at(&mut fixture, 0);
        let second = launched_at(&mut fixture, 1);
        let (dead, live) = if dead_at == 0 {
            (&first, &second)
        } else {
            (&second, &first)
        };
        answer.says(dead.1, ChildStatus::Exited);

        fixture.manager.reap_dead_child();

        assert!(
            !holds(&fixture, &dead.0),
            "the reaper drops the instance whose child the probe calls dead, dead at \
             {dead_at}, recorded: {:?}",
            fixture.manager.instances()
        );
        assert!(
            holds(&fixture, &live.0),
            "and keeps the one it calls alive, dead at {dead_at}, recorded: {:?}",
            fixture.manager.instances()
        );
        assert_eq!(
            counted(&fixture.calls, ChildCall::Reap),
            1,
            "only the dead child is reaped, dead at {dead_at}, calls: {:?}",
            recorded(&fixture.calls)
        );
    }
}

#[test]
fn logs_with_no_instance_read_the_child_the_reaper_left_running() {
    let answer = LivenessAnswer::always(ChildStatus::Running);
    let mut fixture = manager_over_a_gated_listener(
        2,
        answer.clone(),
        Box::new(WatchFreePort),
        always_spawning_config(BOOT_MS),
    );
    let older = launched_at(&mut fixture, 0);
    let newer = launched_at(&mut fixture, 1);
    answer.says(newer.1, ChildStatus::Exited);

    fixture.manager.reap_dead_child();

    let Some(tail) = fixture.manager.child_output(TailLines::default()) else {
        unreachable!(
            "a surviving record still answers the no-instance logs call, recorded: {:?}",
            fixture.manager.instances()
        );
    };
    let Some(surviving) = fixture.ports.first().copied() else {
        unreachable!("the fixture opened its listeners");
    };
    assert_eq!(
        *tail,
        tail_on(surviving),
        "the no-instance logs call reads the surviving child, not the reaped one"
    );
    assert!(
        holds(&fixture, &older.0),
        "the older instance is the one that survived, recorded: {:?}",
        fixture.manager.instances()
    );
}

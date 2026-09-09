use std::sync::{Arc, Mutex};

use cobalt_mcp_server::{
    HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, LaunchPolicy, LifecycleConfig,
    McpPort, OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold, ProbeTimeout, StopOutcome,
    SystemOrphanWatch,
};

use super::support::{StubSpawner, fast_config, free_port, sample_spec, spawn_fake_game};

type StopLog = Arc<Mutex<Vec<McpPort>>>;

struct WatchProbeRecordStop {
    answer:  OrphanStop,
    stopped: StopLog,
}

impl OrphanWatch for WatchProbeRecordStop {
    fn inspect(&self, port: McpPort, timeout: ProbeTimeout) -> PortHold {
        SystemOrphanWatch::new().inspect(port, timeout)
    }

    fn stop(&self, target: OrphanTarget) -> OrphanStop {
        if let Ok(mut log) = self.stopped.lock() {
            log.push(target.port());
        }
        self.answer
    }
}

struct WatchFixedHold {
    hold:    PortHold,
    stopped: StopLog,
}

impl OrphanWatch for WatchFixedHold {
    fn inspect(&self, _port: McpPort, _timeout: ProbeTimeout) -> PortHold {
        self.hold
    }

    fn stop(&self, target: OrphanTarget) -> OrphanStop {
        if let Ok(mut log) = self.stopped.lock() {
            log.push(target.port());
        }
        OrphanStop::Stopped
    }
}

fn recorded(log: &StopLog) -> Vec<McpPort> {
    log.lock().map(|ports| ports.clone()).unwrap_or_default()
}

#[test]
fn a_stop_with_no_owned_child_over_a_live_listener_is_not_not_running() {
    let port = McpPort::new(spawn_fake_game());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Stopped,
            stopped: Arc::clone(&stopped),
        }),
    );

    let outcome = manager.stop(port);

    assert_ne!(outcome, StopOutcome::NotRunning);
    let StopOutcome::OrphanStopped {
        port: reported,
        pid: _,
    } = outcome
    else {
        unreachable!("a held port answers as a stopped orphan, got: {outcome:?}");
    };
    assert_eq!(reported, port);
    assert_eq!(recorded(&stopped), vec![port], "the orphan was stopped");
}

#[test]
fn an_orphan_that_survives_its_stop_is_reported_as_held() {
    let port = McpPort::new(spawn_fake_game());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Survived,
            stopped: Arc::clone(&stopped),
        }),
    );

    let outcome = manager.stop(port);

    let StopOutcome::OrphanHeld {
        port: reported,
        pid: _,
    } = outcome
    else {
        unreachable!("an orphan that survived its stop is reported as held, got: {outcome:?}");
    };
    assert_eq!(reported, port);
    assert_eq!(
        recorded(&stopped),
        vec![port],
        "the stop was attempted before it was reported as held"
    );
}

#[test]
fn a_launch_into_a_held_port_does_not_report_launched() {
    let port = McpPort::new(spawn_fake_game());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Stopped,
            stopped: Arc::clone(&stopped),
        }),
    );

    let outcome = manager.launch(port, &sample_spec());

    let LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { port: reported, .. }) = outcome
    else {
        unreachable!("a held port fails the launch as an orphan, got: {outcome:?}");
    };
    assert_eq!(reported, port);
    assert!(
        recorded(&stopped).is_empty(),
        "a launch reports the orphan, it does not stop it"
    );
}

#[test]
fn a_held_port_answers_the_same_way_under_either_launch_policy() {
    for policy in [LaunchPolicy::Reuse, LaunchPolicy::AlwaysSpawn] {
        let port = McpPort::new(spawn_fake_game());
        let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
        let mut manager = HostManager::with_orphan_watch(
            Box::new(StubSpawner),
            LifecycleConfig::defaults_with_policy(policy),
            Box::new(WatchProbeRecordStop {
                answer:  OrphanStop::Stopped,
                stopped: Arc::clone(&stopped),
            }),
        );

        let launched = manager.launch(port, &sample_spec());
        assert!(
            matches!(
                launched,
                LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { .. })
            ),
            "{policy:?}: a held port fails the launch, got: {launched:?}"
        );

        let stop = manager.stop(port);
        assert!(
            matches!(stop, StopOutcome::OrphanStopped { .. }),
            "{policy:?}: a held port stops the orphan, got: {stop:?}"
        );
    }
}

#[test]
fn an_orphan_with_no_named_process_is_reported_as_held() {
    let port = McpPort::new(free_port());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchFixedHold {
            hold:    PortHold::Orphan(OrphanPid::Unknown),
            stopped: Arc::clone(&stopped),
        }),
    );

    let outcome = manager.stop(port);

    assert_eq!(
        outcome,
        StopOutcome::OrphanHeld {
            port,
            pid: OrphanPid::Unknown,
        }
    );
    assert_eq!(
        recorded(&stopped),
        [],
        "a process that cannot be named is not signalled"
    );
}

#[test]
fn a_free_port_still_answers_not_running() {
    let port = McpPort::new(free_port());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Stopped,
            stopped: Arc::clone(&stopped),
        }),
    );

    assert_eq!(manager.stop(port), StopOutcome::NotRunning);
    assert_eq!(recorded(&stopped), []);
}

#[test]
fn the_shutdown_path_never_adopts_an_orphan() {
    let port = McpPort::new(spawn_fake_game());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Stopped,
            stopped: Arc::clone(&stopped),
        }),
    );

    assert_eq!(manager.stop_owned(), StopOutcome::NotRunning);
    assert_eq!(
        recorded(&stopped),
        [],
        "the shutdown path signalled nothing on port {}",
        *port
    );
    assert!(matches!(
        SystemOrphanWatch::new().inspect(port, fast_config(500).probe_timeout()),
        PortHold::Orphan(_)
    ));
}

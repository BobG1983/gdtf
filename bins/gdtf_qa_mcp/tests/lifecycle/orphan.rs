//! What the REAL manager answers when it owns no child but the port is genuinely held
//! (GTW-926).
//!
//! Every test here drives a real [`HostManager`] against a real loopback listener answering
//! the QA handshake — the state an MCP host restart leaves behind, where the child is alive
//! and listening but this process holds no handle to it. Only the KILL is supplied by the
//! test ([`WatchProbeRecordStop`]), because the listener a test binds is the test process
//! itself: the probe and the pid lookup are the production [`SystemOrphanWatch`], the
//! signal is recorded instead of sent.
//!
//! Before the fix, `stop` in this state answered `NotRunning` and `launch` spawned a second
//! child that raced the orphan for the socket.

use std::sync::{Arc, Mutex};

use gdtf_qa_mcp::{
    HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, OrphanPid, OrphanStop, OrphanTarget,
    OrphanWatch, PortHold, ProbeTimeout, QaHost, QaPort, StopOutcome, SystemOrphanWatch,
};

use super::support::{StubSpawner, fast_config, free_port, spawn_fake_game};

/// The ports a watch was asked to stop, shared between the manager's watch and the test.
type StopLog = Arc<Mutex<Vec<QaPort>>>;

/// A watch that INSPECTS for real and only records the stop.
///
/// `inspect` is [`SystemOrphanWatch`]'s — a real readiness probe against the port, and the
/// real operating-system lookup of who holds it. `stop` records the target and reports the
/// configured answer without signalling anything, because the process holding the port in
/// these tests is the test runner.
struct WatchProbeRecordStop {
    /// What the recorded stop reports back.
    answer:  OrphanStop,
    /// Every port a stop was asked for, in order.
    stopped: StopLog,
}

impl OrphanWatch for WatchProbeRecordStop {
    fn inspect(&self, port: QaPort, timeout: ProbeTimeout) -> PortHold {
        SystemOrphanWatch::new().inspect(port, timeout)
    }

    fn stop(&self, target: OrphanTarget) -> OrphanStop {
        if let Ok(mut log) = self.stopped.lock() {
            log.push(target.port());
        }
        self.answer
    }
}

/// A watch that reports a fixed hold and records the stop — for the branches a real
/// listener cannot produce on demand (a port whose holder cannot be named).
struct WatchFixedHold {
    /// What `inspect` always answers.
    hold:    PortHold,
    /// Every port a stop was asked for, in order.
    stopped: StopLog,
}

impl OrphanWatch for WatchFixedHold {
    fn inspect(&self, _port: QaPort, _timeout: ProbeTimeout) -> PortHold {
        self.hold
    }

    fn stop(&self, target: OrphanTarget) -> OrphanStop {
        if let Ok(mut log) = self.stopped.lock() {
            log.push(target.port());
        }
        OrphanStop::Stopped
    }
}

/// The ports recorded in a stop log.
fn recorded(log: &StopLog) -> Vec<QaPort> {
    log.lock().map(|ports| ports.clone()).unwrap_or_default()
}

/// A manager that owns nothing, over a port a live listener holds, must NOT answer
/// `not_running` — it answers about the orphan, and stops it.
///
/// This is the reported defect: after the MCP host was replaced mid-run, `stop_editor`
/// answered `{"status":"not_running"}` while pid 43744 was still alive and still listening
/// on 7617, and the only way out was a hand-typed `kill`.
#[test]
fn a_stop_with_no_owned_child_over_a_live_listener_is_not_not_running() {
    let port = QaPort::new(spawn_fake_game());
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

/// An orphan that was signalled and is STILL holding the port is reported as HELD, not as
/// stopped.
///
/// The manager reports what the watch observed, so a caller reading `orphan_stopped` knows
/// the port is actually free. Passing a survivor off as stopped would be the same class of
/// unestablished answer as the `not_running` this ticket exists to kill.
#[test]
fn an_orphan_that_survives_its_stop_is_reported_as_held() {
    let port = QaPort::new(spawn_fake_game());
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

/// The same state, seen by a LAUNCH: it reports the orphan holding the port instead of
/// spawning a second child that races it for the socket.
///
/// The spawner here is the same real placeholder-process spawner the other lifecycle tests
/// use, and the listener answers readiness immediately — so before the fix this call
/// returned `Launched` with a pid that had nothing to do with the process on the port.
#[test]
fn a_launch_into_a_held_port_does_not_report_launched() {
    let port = QaPort::new(spawn_fake_game());
    let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(StubSpawner),
        fast_config(500),
        Box::new(WatchProbeRecordStop {
            answer:  OrphanStop::Stopped,
            stopped: Arc::clone(&stopped),
        }),
    );

    let outcome = manager.launch(port, &QaHost::Game.default_spec());

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

/// BOTH hosts get the same treatment: each host's REAL timing config runs the same
/// orphan-aware stop and launch (GTW-926 clause 4). The game manager and the editor manager
/// are the same type, and this is what proves neither is wired to skip the check.
#[test]
fn both_hosts_answer_a_held_port_the_same_way() {
    for host in QaHost::ALL {
        let port = QaPort::new(spawn_fake_game());
        let stopped: StopLog = Arc::new(Mutex::new(Vec::new()));
        let mut manager = HostManager::with_orphan_watch(
            Box::new(StubSpawner),
            host.lifecycle_config(),
            Box::new(WatchProbeRecordStop {
                answer:  OrphanStop::Stopped,
                stopped: Arc::clone(&stopped),
            }),
        );

        let launched = manager.launch(port, &host.default_spec());
        assert!(
            matches!(
                launched,
                LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { .. })
            ),
            "{}: a held port fails the launch, got: {launched:?}",
            host.label()
        );

        let stop = manager.stop(port);
        assert!(
            matches!(stop, StopOutcome::OrphanStopped { .. }),
            "{}: a held port stops the orphan, got: {stop:?}",
            host.label()
        );
    }
}

/// An orphan the operating system cannot name is REPORTED as still holding the port — it is
/// never passed off as stopped, and nothing is signalled.
#[test]
fn an_orphan_with_no_named_process_is_reported_as_held() {
    let port = QaPort::new(free_port());
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
    assert!(
        recorded(&stopped).is_empty(),
        "a process that cannot be named is not signalled"
    );
}

/// A port that nothing answers on still reports `not_running` — the answer the old code
/// gave unconditionally is correct HERE, where it rests on an established fact.
#[test]
fn a_free_port_still_answers_not_running() {
    let port = QaPort::new(free_port());
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
    assert!(recorded(&stopped).is_empty());
}

/// The MCP host's own shutdown path never adopts an orphan: `stop_owned` reports only on
/// the child this manager holds, so a host that owns nothing exits without signalling a
/// listener some other process is responsible for.
#[test]
fn the_shutdown_path_never_adopts_an_orphan() {
    let port = QaPort::new(spawn_fake_game());
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
    assert!(
        recorded(&stopped).is_empty(),
        "the shutdown path signalled nothing on port {}",
        *port
    );
    // The port really was held while that ran — the same state the adopting stop acts on.
    assert!(matches!(
        SystemOrphanWatch::new().inspect(port, fast_config(500).probe_timeout()),
        PortHold::Orphan(_)
    ));
}

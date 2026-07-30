//! A process that ignores the graceful signal is escalated to a kill, and the PORT decides
//! when the stop is done.

use std::{
    net::{Ipv4Addr, TcpListener},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use gdtf_qa_mcp::{ChildPid, OrphanStop, OrphanWatch, PortHold, QaPort, SystemOrphanWatch};

use super::{
    super::support::answer_one,
    placeholder::{EXIT_LIMIT, PROBE, RECHECK, STOP_GRACE, spawn, target_on},
};

/// The signal number `kill -KILL` delivers.
const SIGKILL: i32 = 9;

/// A process that IGNORES the graceful signal keeps its port, so the stop escalates to
/// SIGKILL — and reports `Stopped` only once the port itself stops answering.
///
/// The placeholder ignores SIGTERM and holds a listener that answers until the process is
/// gone, so all three steps are observable: the graceful signal gets its whole grace period
/// (the elapsed time proves the first signal was not already the un-ignorable one), the
/// escalation is what ends the process (the exit signal is SIGKILL), and the answer rests on
/// the port going quiet rather than on a `kill` returning.
#[test]
fn a_process_that_ignores_the_graceful_signal_is_escalated_to_a_kill() {
    let held = spawn_a_listener_held_until_its_process_dies();

    let started = Instant::now();
    let outcome = SystemOrphanWatch::new().stop(target_on(held.port, *held.pid));
    let waited = started.elapsed();

    assert_eq!(outcome, OrphanStop::Stopped);
    assert!(
        waited >= *STOP_GRACE,
        "the graceful signal was given its full grace before the escalation, waited {waited:?}"
    );
    let status = recorded_exit_within(&held.status);
    assert_eq!(
        status.signal(),
        Some(SIGKILL),
        "the process that ignored the graceful signal was killed: {status:?}"
    );
}

/// A port held by a SIGTERM-ignoring process, plus where its exit status lands.
struct HeldPort {
    /// The port the process behind the listener holds.
    port:   QaPort,
    /// The process holding it.
    pid:    ChildPid,
    /// That process's exit status, filled in by the listener thread once it has exited.
    status: Arc<Mutex<Option<ExitStatus>>>,
}

/// Bind a loopback listener that answers the QA handshake only while a SIGTERM-ignoring
/// placeholder process is alive, and stops answering — closing the socket — once it is gone.
///
/// This models the real coupling the stop reads: the port is free exactly when the process
/// behind it is gone. One thread owns both, so the child's exit is observed on the same
/// connection that reports the port quiet.
fn spawn_a_listener_held_until_its_process_dies() -> HeldPort {
    let mut command = Command::new("sh");
    command
        .args(["-c", "trap '' TERM; while :; do sleep 1; done"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = spawn(command);
    let pid = ChildPid::new(child.id());
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    let status: Arc<Mutex<Option<ExitStatus>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&status);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                return;
            };
            // While the child is alive the handshake is answered; the first probe after it
            // exits is closed unanswered, which is how the stop learns the port came free.
            // Returning here drops the listener.
            match child.try_wait() {
                Ok(Some(done)) => {
                    if let Ok(mut slot) = sink.lock() {
                        *slot = Some(done);
                    }
                    return;
                }
                _ => answer_one(&mut stream),
            }
        }
    });
    let held = HeldPort {
        port: QaPort::new(addr.port()),
        pid,
        status,
    };
    await_answering(&held);
    held
}

/// Block until the fixture's listener is actually answering the handshake.
///
/// The listener is bound before its thread runs, so a probe landing in between reads the
/// port as free — the opposite of the state these tests set up. Waiting for the first real
/// answer gives a happens-before edge instead of a race (the same reason the placeholder's
/// stderr line is waited on in [`support`](super::super::support)).
fn await_answering(held: &HeldPort) {
    let deadline = Instant::now() + EXIT_LIMIT;
    let watch = SystemOrphanWatch::new();
    while !matches!(watch.inspect(held.port, PROBE), PortHold::Orphan(_)) {
        assert!(
            Instant::now() < deadline,
            "the fixture's listener answers the handshake on port {}",
            *held.port
        );
        thread::sleep(*RECHECK);
    }
}

/// The exit status the listener thread recorded, once it has one.
fn recorded_exit_within(status: &Arc<Mutex<Option<ExitStatus>>>) -> ExitStatus {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        if let Ok(slot) = status.lock()
            && let Some(done) = *slot
        {
            return done;
        }
        assert!(
            Instant::now() < deadline,
            "the placeholder process behind the held port exited"
        );
        thread::sleep(*RECHECK);
    }
}

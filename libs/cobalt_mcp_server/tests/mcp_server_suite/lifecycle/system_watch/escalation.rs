use std::{
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use cobalt_mcp_server::{
    ChildPid, McpPort, OrphanEvent, OrphanStop, OrphanWatch, PortHold, SystemOrphanWatch,
};

use super::{
    super::support::answer_one,
    placeholder::{PROBE, spawn, target_on},
};
use crate::ports::{bind_loopback, port_of};

const SIGKILL: i32 = 9;

#[test]
fn a_process_that_ignores_the_graceful_signal_is_escalated_to_a_kill() {
    let held = spawn_a_listener_held_until_its_process_dies();

    let escalation = SystemOrphanWatch::new().stop_recorded(target_on(held.port, *held.pid));

    assert_eq!(escalation.outcome(), OrphanStop::Stopped);
    let events = escalation.events();
    let Some(kill_at) = events
        .iter()
        .position(|event| *event == OrphanEvent::KillSent)
    else {
        unreachable!("the process that ignored the graceful signal was killed, steps: {events:?}");
    };
    assert_eq!(
        events.first(),
        Some(&OrphanEvent::TerminateSent),
        "the graceful signal goes out first, steps: {events:?}"
    );
    assert!(
        events[..kill_at].contains(&OrphanEvent::ProbeHeld),
        "the kill follows a poll that found the port still held, steps: {events:?}"
    );
    let status = recorded_exit(&held.status);
    assert_eq!(
        status.signal(),
        Some(SIGKILL),
        "the process that ignored the graceful signal was killed: {status:?}"
    );
}

struct HeldPort {
    port:   McpPort,
    pid:    ChildPid,
    status: Arc<Mutex<Option<ExitStatus>>>,
}

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
    let listener = bind_loopback();
    let port = port_of(&listener);
    let status: Arc<Mutex<Option<ExitStatus>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&status);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                return;
            };
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
        port: McpPort::new(port),
        pid,
        status,
    };
    await_answering(&held);
    held
}

// No deadline: the fixture's listener always comes up, load only delays it.
fn await_answering(held: &HeldPort) {
    let watch = SystemOrphanWatch::new();
    while !matches!(watch.inspect(held.port, PROBE), PortHold::Orphan(_)) {
        thread::yield_now();
    }
}

// No deadline: the kill always lands and the fixture thread records it.
fn recorded_exit(status: &Arc<Mutex<Option<ExitStatus>>>) -> ExitStatus {
    loop {
        if let Ok(slot) = status.lock()
            && let Some(done) = *slot
        {
            return done;
        }
        thread::yield_now();
    }
}

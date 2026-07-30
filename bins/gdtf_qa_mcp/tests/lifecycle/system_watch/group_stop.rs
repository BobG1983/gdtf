//! The stop reaches a process the named one SPAWNED, not just the named one.
//!
//! This is the half only the process-group target can carry, and it is the reason the real
//! child is launched in a group of its own: a `cargo run` launcher spawns the app and stays
//! in the same group, so a stop that signalled only the pid it was handed would kill the
//! launcher and leave the app running — still holding the QA port, which is the whole defect
//! this ticket is about. Dropping the group target from
//! [`SystemOrphanWatch::stop`](gdtf_qa_mcp::SystemOrphanWatch) fails this test and no other.

use std::{
    io::{BufRead, BufReader},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    thread,
    time::Instant,
};

use gdtf_qa_mcp::{OrphanStop, OrphanWatch, QaPort, SystemOrphanWatch};

use super::{
    super::support::free_port,
    placeholder::{EXIT_LIMIT, RECHECK, group_of, spawn, still_listed, target_on},
};

/// A launcher placeholder leading its own process group, plus the pid of the process it
/// spawned into that same group.
struct Launcher {
    /// The group leader — the process the stop is actually told about.
    leader:  Child,
    /// The process the leader spawned, which the stop is never told about.
    spawned: u32,
}

/// Stopping the leader of a group also stops what that leader spawned into the group.
///
/// The assertion is about the SPAWNED process: the leader dies either way (the bare pid
/// reaches it), so only the spawned process discriminates between signalling the group and
/// signalling one pid.
#[test]
fn the_real_stop_reaches_a_process_the_group_leader_spawned() {
    let mut launcher = spawn_a_leader_with_a_process_of_its_own();
    let leader = launcher.leader.id();
    assert_eq!(group_of(leader), leader, "the launcher leads its own group");
    assert_ne!(
        launcher.spawned, leader,
        "the spawned process is a process of its own, not the launcher"
    );
    assert_eq!(
        group_of(launcher.spawned),
        leader,
        "the spawned process sits in the launcher's group, the way a launched app does"
    );

    let outcome = SystemOrphanWatch::new().stop(target_on(QaPort::new(free_port()), leader));

    assert_eq!(outcome, OrphanStop::Stopped);
    await_gone(launcher.spawned);
    drop(launcher.leader.wait());
}

/// Spawn a `sh` in a process group of its own that starts a long-running process in that
/// same group and reports its pid on stdout, then waits — the shape a launcher leaves.
///
/// The pid is read back before the test returns, so the spawned process is known to exist
/// (and to be readable by `ps`) before anything is signalled.
fn spawn_a_leader_with_a_process_of_its_own() -> Launcher {
    let mut command = Command::new("sh");
    command
        .args(["-c", "sleep 30 & echo $!; wait"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let mut leader = spawn(command);
    let Some(stdout) = leader.stdout.take() else {
        unreachable!("the launcher placeholder was spawned with a stdout pipe");
    };
    let mut line = String::new();
    let Ok(read) = BufReader::new(stdout).read_line(&mut line) else {
        unreachable!("the launcher placeholder's stdout can be read");
    };
    assert!(read > 0, "the launcher placeholder reported a pid");
    let Ok(spawned) = line.trim().parse::<u32>() else {
        unreachable!("the launcher placeholder reports a numeric pid, got: {line:?}");
    };
    Launcher { leader, spawned }
}

/// Wait until the operating system no longer lists `pid`, failing loudly if it never stops.
fn await_gone(pid: u32) {
    let deadline = Instant::now() + EXIT_LIMIT;
    while still_listed(pid) {
        assert!(
            Instant::now() < deadline,
            "the process the launcher spawned is still running after the stop (pid {pid})"
        );
        thread::sleep(*RECHECK);
    }
}

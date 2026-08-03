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

struct Launcher {
        leader:  Child,
        spawned: u32,
}

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

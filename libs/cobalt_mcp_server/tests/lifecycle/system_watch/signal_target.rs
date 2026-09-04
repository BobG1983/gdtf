use std::os::unix::process::ExitStatusExt;

use cobalt_mcp_server::{OrphanStop, OrphanWatch, QaPort, SystemOrphanWatch};

use super::{
    super::support::free_port,
    placeholder::{
        exit_status, group_of, sleeper_command, spawn, spawn_sleeper_in_its_own_group, target_on,
    },
};

#[test]
fn the_real_stop_reaches_a_process_that_is_not_a_group_leader() {
    let mut child = spawn(sleeper_command());
    let pid = child.id();
    assert_ne!(
        group_of(pid),
        pid,
        "the placeholder is not a process-group leader, so only the bare pid reaches it"
    );

    let outcome = SystemOrphanWatch::new().stop(target_on(QaPort::new(free_port()), pid));

    assert_eq!(outcome, OrphanStop::Stopped);
    let status = exit_status(&mut child);
    assert!(
        status.signal().is_some(),
        "the placeholder was signalled, not left to finish: {status:?}"
    );
}

#[test]
fn the_real_stop_reaches_a_process_group_leader() {
    let mut child = spawn_sleeper_in_its_own_group();
    let pid = child.id();
    assert_eq!(group_of(pid), pid, "the placeholder leads its own group");

    let outcome = SystemOrphanWatch::new().stop(target_on(QaPort::new(free_port()), pid));

    assert_eq!(outcome, OrphanStop::Stopped);
    let status = exit_status(&mut child);
    assert!(
        status.signal().is_some(),
        "the placeholder was signalled, not left to finish: {status:?}"
    );
}

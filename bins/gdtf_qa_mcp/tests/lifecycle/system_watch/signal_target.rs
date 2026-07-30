//! The real stop reaches the process it was given, whether or not that process leads a
//! process group of its own.
//!
//! These two tests pin the NAMED process dying. The other half — a process the named one
//! spawned, which only the group target can carry — is [`group_stop`](super::group_stop).

use std::os::unix::process::ExitStatusExt;

use gdtf_qa_mcp::{OrphanStop, OrphanWatch, QaPort, SystemOrphanWatch};

use super::{
    super::support::free_port,
    placeholder::{
        exit_status_within, group_of, sleeper_command, spawn, spawn_sleeper_in_its_own_group,
        target_on,
    },
};

/// The real stop kills a process that is NOT the leader of its own process group — the
/// exact shape the bug report showed, where the launcher had already exited and the
/// surviving listener was re-parented to init with no group carrying its id.
///
/// The placeholder here is spawned WITHOUT its own group, so it inherits the test runner's:
/// its pid names no process group at all (a group id only exists while its leader does), so
/// a stop that signalled only the group would leave it running.
#[test]
fn the_real_stop_reaches_a_process_that_is_not_a_group_leader() {
    let mut child = spawn(sleeper_command()); // No `process_group`: it inherits ours.
    let pid = child.id();
    assert_ne!(
        group_of(pid),
        pid,
        "the placeholder is not a process-group leader, so only the bare pid reaches it"
    );

    let outcome = SystemOrphanWatch::new().stop(target_on(QaPort::new(free_port()), pid));

    assert_eq!(outcome, OrphanStop::Stopped);
    let status = exit_status_within(&mut child);
    assert!(
        status.signal().is_some(),
        "the placeholder was signalled, not left to finish: {status:?}"
    );
}

/// The real stop also kills a process that IS its own group's leader — the shape a launched
/// child has, since [`ProcessChild`](gdtf_qa_mcp::ProcessChild) puts every child in a group
/// of its own.
#[test]
fn the_real_stop_reaches_a_process_group_leader() {
    let mut child = spawn_sleeper_in_its_own_group();
    let pid = child.id();
    assert_eq!(group_of(pid), pid, "the placeholder leads its own group");

    let outcome = SystemOrphanWatch::new().stop(target_on(QaPort::new(free_port()), pid));

    assert_eq!(outcome, OrphanStop::Stopped);
    let status = exit_status_within(&mut child);
    assert!(
        status.signal().is_some(),
        "the placeholder was signalled, not left to finish: {status:?}"
    );
}

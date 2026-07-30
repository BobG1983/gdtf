//! Spawning, process-group reading and exit-waiting helpers shared by the watch tests.
//!
//! The placeholder processes here are the only things these tests ever signal: a `sh` that
//! sleeps until something stops it, spawned either in the test runner's process group or in
//! one of its own, plus the short timing knobs that keep the real stop's grace period quick.

use core::time::Duration;
use std::{
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::Instant,
};

use gdtf_qa_mcp::{ChildPid, KillGrace, OrphanTarget, PollInterval, ProbeTimeout, QaPort};

/// How long the real stop waits before escalating — short, so these tests stay quick.
pub(super) const STOP_GRACE: KillGrace = KillGrace::new(Duration::from_millis(150));

/// The per-probe socket deadline the real stop re-checks the port with.
pub(super) const PROBE: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(300));

/// How long a test waits for a signalled process to actually be gone.
pub(super) const EXIT_LIMIT: Duration = Duration::from_secs(10);

/// How often anything here re-checks while waiting: the stop's port re-probe, the tests' polls.
pub(super) const RECHECK: PollInterval = PollInterval::new(Duration::from_millis(5));

/// A stop target on `port` for `pid`, with the short timing knobs these tests use.
pub(super) const fn target_on(port: QaPort, pid: u32) -> OrphanTarget {
    OrphanTarget::new(port, ChildPid::new(pid), STOP_GRACE, PROBE, RECHECK)
}

/// The placeholder process every test here signals: it sleeps until something stops it.
pub(super) fn sleeper_command() -> Command {
    let mut command = Command::new("sh");
    command
        .args(["-c", "exec sleep 30"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Spawn the placeholder in a process group of its own, the way a real child is launched.
pub(super) fn spawn_sleeper_in_its_own_group() -> Child {
    let mut command = sleeper_command();
    command.process_group(0);
    spawn(command)
}

/// Spawn `command`, failing the test loudly if the operating system will not.
pub(super) fn spawn(mut command: Command) -> Child {
    let Ok(child) = command.spawn() else {
        unreachable!("the test can spawn a placeholder process");
    };
    child
}

/// The process group id the operating system reports for `pid`, via `ps(1)`.
pub(super) fn group_of(pid: u32) -> u32 {
    let Ok(text) = ps_field("pgid=", pid) else {
        unreachable!("the test can run ps(1) to read a process group");
    };
    let Ok(group) = text.trim().parse::<u32>() else {
        unreachable!("ps(1) reports a numeric process group, got: {text:?}");
    };
    group
}

/// Whether the operating system still lists `pid` as a process, via `ps(1)`.
///
/// A process signalled out of existence stops being listed once it is reaped; a placeholder
/// whose parent this stop killed is reparented to init, which reaps it, so the listing going
/// empty is the observable end state.
pub(super) fn still_listed(pid: u32) -> bool {
    ps_field("pid=", pid).is_ok_and(|text| !text.trim().is_empty())
}

/// Read one `ps(1)` field for `pid`; `Err` when `ps` itself could not be run or read.
fn ps_field(field: &str, pid: u32) -> Result<String, ()> {
    let Ok(output) = Command::new("ps")
        .args(["-o", field, "-p", &format!("{pid}")])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return Err(());
    };
    String::from_utf8(output.stdout).map_err(|_| ())
}

/// Wait for `child` to exit and report its status.
pub(super) fn exit_status_within(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) => {}
            Err(_) => unreachable!("the placeholder process can be waited on"),
        }
        assert!(
            Instant::now() < deadline,
            "the placeholder process is still running after the stop"
        );
        thread::sleep(*RECHECK);
    }
}

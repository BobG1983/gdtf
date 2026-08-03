use core::time::Duration;
use std::{
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::Instant,
};

use gdtf_qa_mcp::{ChildPid, KillGrace, OrphanTarget, PollInterval, ProbeTimeout, QaPort};

pub(super) const STOP_GRACE: KillGrace = KillGrace::new(Duration::from_millis(150));

pub(super) const PROBE: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(300));

pub(super) const EXIT_LIMIT: Duration = Duration::from_secs(10);

pub(super) const RECHECK: PollInterval = PollInterval::new(Duration::from_millis(5));

pub(super) const fn target_on(port: QaPort, pid: u32) -> OrphanTarget {
    OrphanTarget::new(port, ChildPid::new(pid), STOP_GRACE, PROBE, RECHECK)
}

pub(super) fn sleeper_command() -> Command {
    let mut command = Command::new("sh");
    command
        .args(["-c", "exec sleep 30"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(super) fn spawn_sleeper_in_its_own_group() -> Child {
    let mut command = sleeper_command();
    command.process_group(0);
    spawn(command)
}

pub(super) fn spawn(mut command: Command) -> Child {
    let Ok(child) = command.spawn() else {
        unreachable!("the test can spawn a placeholder process");
    };
    child
}

pub(super) fn group_of(pid: u32) -> u32 {
    let Ok(text) = ps_field("pgid=", pid) else {
        unreachable!("the test can run ps(1) to read a process group");
    };
    let Ok(group) = text.trim().parse::<u32>() else {
        unreachable!("ps(1) reports a numeric process group, got: {text:?}");
    };
    group
}

pub(super) fn still_listed(pid: u32) -> bool {
    ps_field("pid=", pid).is_ok_and(|text| !text.trim().is_empty())
}

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

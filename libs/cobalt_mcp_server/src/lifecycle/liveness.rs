//! Ask the OS whether a recorded child process is still running.

use std::process::{Command, Stdio};

use super::values::{ChildPid, ChildStatus};

/// Answers whether a child pid is still a running process.
pub trait ChildLiveness {
    /// Status of `pid` as the operating system reports it.
    fn status(&self, pid: ChildPid) -> ChildStatus;
}

/// Liveness read from the OS process table.
pub struct SystemLiveness;

impl SystemLiveness {
    /// Create the system probe.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SystemLiveness {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildLiveness for SystemLiveness {
    fn status(&self, pid: ChildPid) -> ChildStatus {
        process_state(pid)
    }
}

// An exited child that has not been reaped shows state Z, not a missing process.
#[cfg(unix)]
fn process_state(pid: ChildPid) -> ChildStatus {
    let target = format!("{}", *pid);
    let Ok(output) = Command::new("ps")
        .args(["-o", "state=", "-p", target.as_str()])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return ChildStatus::Exited;
    };
    if !output.status.success() {
        return ChildStatus::Exited;
    }
    let Ok(text) = String::from_utf8(output.stdout) else {
        return ChildStatus::Exited;
    };
    match text.trim().chars().next() {
        Some('Z') | None => ChildStatus::Exited,
        Some(_) => ChildStatus::Running,
    }
}

#[cfg(not(unix))]
fn process_state(_pid: ChildPid) -> ChildStatus {
    ChildStatus::Running
}

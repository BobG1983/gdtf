//! Detect and stop processes holding a QA port that we did not launch.

use std::process::{Command, Stdio};

use cobalt_mcp_protocol::ports::McpPort;

use super::{
    config::LifecycleConfig,
    probe::probe_ready,
    values::{ChildPid, KillGrace, PollInterval, ProbeTimeout, Readiness},
};

/// Whether a port is free or held by an unknown process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortHold {
    /// Nothing listening.
    Free,
    /// Something else is listening.
    Orphan(OrphanPid),
}

/// Pid of an orphan, if known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanPid {
    /// Resolved via lsof (or similar).
    Known(ChildPid),
    /// Could not resolve.
    Unknown,
}

/// Result of trying to stop an orphan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanStop {
    /// Port is free now.
    Stopped,
    /// Still listening after kill attempts.
    Survived,
}

/// Parameters for stopping a known orphan pid on a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanTarget {
    port:  McpPort,
    pid:   ChildPid,
    grace: KillGrace,
    probe: ProbeTimeout,
    poll:  PollInterval,
}

impl OrphanTarget {
    /// Build from explicit fields.
    #[must_use]
    pub const fn new(
        port: McpPort,
        pid: ChildPid,
        grace: KillGrace,
        probe: ProbeTimeout,
        poll: PollInterval,
    ) -> Self {
        Self {
            port,
            pid,
            grace,
            probe,
            poll,
        }
    }

    /// Build from a lifecycle config.
    #[must_use]
    pub const fn from_config(port: McpPort, pid: ChildPid, config: LifecycleConfig) -> Self {
        Self::new(
            port,
            pid,
            config.kill_grace(),
            config.probe_timeout(),
            config.poll_interval(),
        )
    }

    /// Port held by the orphan.
    #[must_use]
    pub const fn port(&self) -> McpPort {
        self.port
    }

    /// Orphan process id.
    #[must_use]
    pub const fn pid(&self) -> ChildPid {
        self.pid
    }

    /// Kill grace period.
    #[must_use]
    pub const fn grace(&self) -> KillGrace {
        self.grace
    }

    /// Probe timeout while waiting for the port to free.
    #[must_use]
    pub const fn probe(&self) -> ProbeTimeout {
        self.probe
    }

    /// Poll interval while waiting.
    #[must_use]
    pub const fn poll(&self) -> PollInterval {
        self.poll
    }
}

/// Inspect and stop orphans on a port.
pub trait OrphanWatch {
    /// Check whether something is listening on `port`.
    fn inspect(&self, port: McpPort, timeout: ProbeTimeout) -> PortHold;

    /// Attempt to stop a known orphan.
    fn stop(&self, target: OrphanTarget) -> OrphanStop;
}

/// System implementation using probe + `kill` / `lsof` on Unix.
pub struct SystemOrphanWatch;

impl SystemOrphanWatch {
    /// Create the system watch.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SystemOrphanWatch {
    fn default() -> Self {
        Self::new()
    }
}

impl OrphanWatch for SystemOrphanWatch {
    fn inspect(&self, port: McpPort, timeout: ProbeTimeout) -> PortHold {
        match probe_ready(port, timeout) {
            Readiness::NotYet => PortHold::Free,
            Readiness::Ready => PortHold::Orphan(pid_listening_on(port)),
        }
    }

    fn stop(&self, target: OrphanTarget) -> OrphanStop {
        signal(target.pid(), StopSignal::Terminate);
        if matches!(wait_until_free(&target), OrphanStop::Stopped) {
            return OrphanStop::Stopped;
        }
        signal(target.pid(), StopSignal::Kill);
        wait_until_free(&target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopSignal {
    Terminate,
    Kill,
}

impl StopSignal {
    const fn flag(self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Kill => "-KILL",
        }
    }
}

fn wait_until_free(target: &OrphanTarget) -> OrphanStop {
    let deadline = std::time::Instant::now() + *target.grace();
    loop {
        if matches!(
            probe_ready(target.port(), target.probe()),
            Readiness::NotYet
        ) {
            return OrphanStop::Stopped;
        }
        if std::time::Instant::now() >= deadline {
            return OrphanStop::Survived;
        }
        std::thread::sleep(*target.poll());
    }
}

#[cfg(unix)]
fn signal(pid: ChildPid, stop: StopSignal) {
    for target in [format!("-{}", *pid), format!("{}", *pid)] {
        let mut command = Command::new("kill");
        command
            .args([stop.flag(), target.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(mut helper) = command.spawn() {
            drop(helper.wait());
        }
    }
}

#[cfg(not(unix))]
fn signal(_pid: ChildPid, _stop: StopSignal) {}

#[cfg(unix)]
fn pid_listening_on(port: McpPort) -> OrphanPid {
    let selector = format!("-iTCP@127.0.0.1:{}", *port);
    let Ok(output) = Command::new("lsof")
        .args(["-nP", "-t", selector.as_str(), "-sTCP:LISTEN"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return OrphanPid::Unknown;
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return OrphanPid::Unknown;
    };
    text.lines()
        .find_map(|line| line.trim().parse::<u32>().ok())
        .map_or(OrphanPid::Unknown, |pid| {
            OrphanPid::Known(ChildPid::new(pid))
        })
}

#[cfg(not(unix))]
fn pid_listening_on(_port: McpPort) -> OrphanPid {
    OrphanPid::Unknown
}

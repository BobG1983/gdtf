use std::process::{Command, Stdio};

use super::{
    config::LifecycleConfig,
    probe::probe_ready,
    values::{ChildPid, KillGrace, PollInterval, ProbeTimeout, Readiness},
};
use crate::link::QaPort;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortHold {
        Free,
        Orphan(OrphanPid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanPid {
        Known(ChildPid),
            Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanStop {
        Stopped,
        Survived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanTarget {
        port:  QaPort,
        pid:   ChildPid,
        grace: KillGrace,
        probe: ProbeTimeout,
        poll:  PollInterval,
}

impl OrphanTarget {
        #[must_use]
    pub const fn new(
        port: QaPort,
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

            #[must_use]
    pub const fn from_config(port: QaPort, pid: ChildPid, config: LifecycleConfig) -> Self {
        Self::new(
            port,
            pid,
            config.kill_grace(),
            config.probe_timeout(),
            config.poll_interval(),
        )
    }

        #[must_use]
    pub const fn port(&self) -> QaPort {
        self.port
    }

        #[must_use]
    pub const fn pid(&self) -> ChildPid {
        self.pid
    }

        #[must_use]
    pub const fn grace(&self) -> KillGrace {
        self.grace
    }

        #[must_use]
    pub const fn probe(&self) -> ProbeTimeout {
        self.probe
    }

        #[must_use]
    pub const fn poll(&self) -> PollInterval {
        self.poll
    }
}

pub trait OrphanWatch {
        fn inspect(&self, port: QaPort, timeout: ProbeTimeout) -> PortHold;

        fn stop(&self, target: OrphanTarget) -> OrphanStop;
}

pub struct SystemOrphanWatch;

impl SystemOrphanWatch {
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
    fn inspect(&self, port: QaPort, timeout: ProbeTimeout) -> PortHold {
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
fn pid_listening_on(port: QaPort) -> OrphanPid {
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
fn pid_listening_on(_port: QaPort) -> OrphanPid {
    OrphanPid::Unknown
}

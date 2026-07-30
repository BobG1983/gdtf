//! Finding — and stopping — a child on this host's port that this host never spawned
//! (GTW-926).
//!
//! The manager's only record of a child is the owned handle it holds in memory, so a host
//! process that is REPLACED mid-run (every `/mcp` reconnect does exactly that) starts up
//! owning nothing while the previous host's child is still alive, still listening, and
//! still holding the fixed loopback port. That child is an ORPHAN: nothing in this process
//! can reach it through a handle, and it deliberately survives its parent (the child is put
//! in its own process group, so a signal to the host's group never reaches it).
//!
//! [`OrphanWatch`] is how the manager asks the world about that state instead of assuming
//! it: [`inspect`](OrphanWatch::inspect) answers whether a port is [`Free`](PortHold::Free)
//! or held by an orphan, and [`stop`](OrphanWatch::stop) stops one. Two things must both be
//! true before a process is treated as an orphan: the port answers the QA protocol's
//! `Hello` (so it is a gdtf QA listener, not some unrelated program), and the operating
//! system can name the process behind it. [`SystemOrphanWatch`] is the real implementation;
//! the trait is what lets a test drive the manager's decisions without signalling anything.

use std::process::{Command, Stdio};

use super::{
    config::LifecycleConfig,
    probe::probe_ready,
    values::{ChildPid, KillGrace, PollInterval, ProbeTimeout, Readiness},
};
use crate::link::QaPort;

/// Who, if anyone, holds a host port that this manager does not own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortHold {
    /// Nothing is answering the QA protocol on the port — a launch may take it.
    Free,
    /// A QA listener this host did not spawn is answering on the port.
    Orphan(OrphanPid),
}

/// Whether the operating system could name the process behind an orphan listener.
///
/// A typed answer rather than an `Option<ChildPid>` at the call sites: the difference
/// decides whether the orphan can be stopped at all, and it is reported to the caller
/// either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanPid {
    /// The process id holding the port.
    Known(ChildPid),
    /// The port answers, but no process id could be resolved for it — nothing here can
    /// signal it.
    Unknown,
}

/// What happened when an orphan was asked to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrphanStop {
    /// The orphan is gone and its port is free again.
    Stopped,
    /// The orphan is still holding the port.
    Survived,
}

/// One orphan to stop: the port it holds, the process holding it, and the three timing
/// knobs the stop waits on.
///
/// A plain aggregate (not a newtype — it has more than one field) so [`OrphanWatch::stop`]
/// takes one argument rather than five positional ones. The production path builds one
/// through [`from_config`](Self::from_config), so nothing here waits on a duration the
/// manager's [`LifecycleConfig`] cannot reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanTarget {
    /// The port the orphan is listening on.
    port:  QaPort,
    /// The process holding that port.
    pid:   ChildPid,
    /// How long to wait after the graceful signal before escalating.
    grace: KillGrace,
    /// The per-probe socket deadline used to re-check the port.
    probe: ProbeTimeout,
    /// How long the port re-check sleeps between probes.
    poll:  PollInterval,
}

impl OrphanTarget {
    /// Build a stop target from the port, the process holding it, and the timing knobs.
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

    /// Build a stop target from the port, the process holding it, and the manager's timing
    /// config — the production path, so no knob on this path is hardcoded past the config.
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

    /// The port the orphan is listening on.
    #[must_use]
    pub const fn port(&self) -> QaPort {
        self.port
    }

    /// The process holding that port.
    #[must_use]
    pub const fn pid(&self) -> ChildPid {
        self.pid
    }

    /// How long the stop waits after the graceful signal before escalating.
    #[must_use]
    pub const fn grace(&self) -> KillGrace {
        self.grace
    }

    /// The per-probe socket deadline used to re-check the port.
    #[must_use]
    pub const fn probe(&self) -> ProbeTimeout {
        self.probe
    }

    /// How long the port re-check sleeps between probes.
    #[must_use]
    pub const fn poll(&self) -> PollInterval {
        self.poll
    }
}

/// How the manager learns about, and stops, a child it does not own.
///
/// The manager depends on this trait, not on [`SystemOrphanWatch`], so the orphan-handling
/// decisions can be exercised against a real listener without any test signalling a real
/// process.
pub trait OrphanWatch {
    /// Whether `port` is free or held by a QA listener this host did not spawn.
    fn inspect(&self, port: QaPort, timeout: ProbeTimeout) -> PortHold;

    /// Stop the orphan named by `target` and report whether its port came free.
    fn stop(&self, target: OrphanTarget) -> OrphanStop;
}

/// The real [`OrphanWatch`] — the QA readiness probe plus the operating system's own
/// answer to "which process is listening here".
pub struct SystemOrphanWatch;

impl SystemOrphanWatch {
    /// Build the real watch.
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

/// Which stop signal to deliver to an orphan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopSignal {
    /// SIGTERM — the graceful "please exit" request.
    Terminate,
    /// SIGKILL — the un-catchable forceful stop.
    Kill,
}

impl StopSignal {
    /// The `kill(1)` flag that names this signal.
    const fn flag(self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Kill => "-KILL",
        }
    }
}

/// Re-probe the port until it stops answering or the grace period runs out.
///
/// The orphan is not this process's child, so there is no handle to wait on and nothing to
/// reap (it was re-parented to init, which reaps it). The port going quiet is the observable
/// fact that it is gone.
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

/// Deliver a stop signal to the orphan.
///
/// Both the orphan's process GROUP and the process itself are signalled. The group covers
/// the case where the orphan is still the leader of the group its launch created — killing
/// only the `cargo run` launcher would leave the app it spawned behind — and the bare pid
/// covers the case the bug report actually showed, where the launcher had already exited
/// and the surviving listener is no longer a group leader, so no group carries its id. The
/// workspace denies `unsafe_code`, so this shells out to `kill(1)` rather than calling
/// `kill(2)`; each short-lived helper is waited on so it does not linger as a zombie.
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

/// Non-Unix fallback: no `kill(1)`, so an orphan is reported rather than stopped.
#[cfg(not(unix))]
fn signal(_pid: ChildPid, _stop: StopSignal) {}

/// The process listening on `port`, as the operating system reports it.
///
/// Shells out to `lsof(8)` — the one portable-enough answer available without `unsafe`
/// syscalls or a new dependency. Anything unexpected (no `lsof`, no match, unparsable
/// output) is [`Unknown`](OrphanPid::Unknown), which the manager reports rather than acting
/// on.
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

/// Non-Unix fallback: no `lsof`, so the holder is never named.
#[cfg(not(unix))]
fn pid_listening_on(_port: QaPort) -> OrphanPid {
    OrphanPid::Unknown
}

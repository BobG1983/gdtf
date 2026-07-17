//! The small value types the game-lifecycle layer talks in (GTW-745).
//!
//! Newtypes over the plumbing primitives (no-bare-types): the child's [`ChildPid`], the
//! captured [`StderrTail`], a [`SpawnError`] message, and the four `Duration` knobs the
//! launch/stop logic waits on. Plus the two status answers the polling loops read —
//! [`ChildStatus`] (has the child exited?) and [`Readiness`] (is the game answering?).

use core::{ops::Deref, time::Duration};

/// A launched child process's operating-system **process id**.
///
/// Private-inner newtype over `u32` (no-bare-types). Captured once at spawn and reported
/// back so a caller can see which process was started or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChildPid(u32);

impl ChildPid {
    /// Build a process id from the raw value the operating system assigned.
    #[must_use]
    pub const fn new(pid: u32) -> Self {
        Self(pid)
    }
}

impl Deref for ChildPid {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The tail of a failed child's captured standard-error output.
///
/// Private-inner newtype over `String` (no-bare-types). When a launch fails, this carries
/// the last lines the child wrote to stderr so a caller sees WHY it never came up, instead
/// of a bare "boot failed".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StderrTail(String);

impl StderrTail {
    /// Build a stderr tail from the captured text.
    #[must_use]
    pub const fn new(tail: String) -> Self {
        Self(tail)
    }
}

impl Deref for StderrTail {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The reason a child could not be spawned AT ALL — the spawn call itself failing (e.g.
/// the launcher program was not found), as distinct from a child that started but never
/// became ready.
///
/// Private-inner newtype over `String` (no-bare-types) carrying the underlying
/// `io::Error`'s display text (the cause, not a domain value).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpawnError(String);

impl SpawnError {
    /// Build a spawn-failure message from the underlying cause's text.
    #[must_use]
    pub const fn new(reason: String) -> Self {
        Self(reason)
    }
}

impl Deref for SpawnError {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// How long a launch waits for the child to answer the readiness handshake before it
/// gives up, kills the orphan, and reports a boot failure.
///
/// Private-inner newtype over [`Duration`] (no-bare-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootTimeout(Duration);

impl BootTimeout {
    /// Build a boot timeout from its duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

impl Deref for BootTimeout {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// How long the launch loop sleeps between readiness probes.
///
/// Private-inner newtype over [`Duration`] (no-bare-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollInterval(Duration);

impl PollInterval {
    /// Build a poll interval from its duration.
    #[must_use]
    pub const fn new(interval: Duration) -> Self {
        Self(interval)
    }
}

impl Deref for PollInterval {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// How long a graceful stop waits after SIGTERM before escalating to SIGKILL.
///
/// Private-inner newtype over [`Duration`] (no-bare-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KillGrace(Duration);

impl KillGrace {
    /// Build a kill grace period from its duration.
    #[must_use]
    pub const fn new(grace: Duration) -> Self {
        Self(grace)
    }
}

impl Deref for KillGrace {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The connect / read / write deadline set on one readiness-probe socket, so a probe
/// against a half-open port cannot block the launch loop.
///
/// Private-inner newtype over [`Duration`] (no-bare-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProbeTimeout(Duration);

impl ProbeTimeout {
    /// Build a probe timeout from its duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

impl Deref for ProbeTimeout {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Whether a managed child is still running or has exited — the answer a `try_wait`-style
/// poll returns (a typed answer rather than a bare `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildStatus {
    /// The child is still alive.
    Running,
    /// The child has exited (and can be reaped).
    Exited,
}

/// Whether the game's `net_qa` listener is answering yet — the answer one readiness probe
/// returns (a typed answer rather than a bare `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Readiness {
    /// The listener answered a decodable reply — the game is up.
    Ready,
    /// The port refused, timed out, or returned garbage — not up yet.
    NotYet,
}

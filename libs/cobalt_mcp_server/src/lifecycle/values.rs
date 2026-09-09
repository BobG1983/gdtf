//! Small value types for child lifecycle.

use core::{ops::Deref, time::Duration};

use cobalt_mcp_protocol::ports::McpPort;

/// OS process id of a managed child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChildPid(u32);

impl ChildPid {
    /// Wrap a pid.
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

/// Name a host gives one child it records, unique within that host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceId(String);

impl InstanceId {
    /// Wrap an id.
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self(id)
    }
}

impl Deref for InstanceId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One child a host records: its id, the port it listens on, and its pid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedInstance {
    id:   InstanceId,
    port: McpPort,
    pid:  ChildPid,
}

impl RecordedInstance {
    /// Build from the three facts a host records.
    #[must_use]
    pub const fn new(id: InstanceId, port: McpPort, pid: ChildPid) -> Self {
        Self { id, port, pid }
    }

    /// Id this host knows the child by.
    #[must_use]
    pub const fn id(&self) -> &InstanceId {
        &self.id
    }

    /// Port the child listens on.
    #[must_use]
    pub const fn port(&self) -> McpPort {
        self.port
    }

    /// Process id of the child.
    #[must_use]
    pub const fn pid(&self) -> ChildPid {
        self.pid
    }
}

/// Captured stderr/stdout tail when a launch fails.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FailureTail(String);

impl FailureTail {
    /// Wrap the captured text.
    #[must_use]
    pub const fn new(tail: String) -> Self {
        Self(tail)
    }
}

impl Deref for FailureTail {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Recent child process output for the logs tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OutputTail(String);

impl OutputTail {
    /// Wrap the captured text.
    #[must_use]
    pub const fn new(tail: String) -> Self {
        Self(tail)
    }
}

impl Deref for OutputTail {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// How many trailing log lines to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TailLines(usize);

impl TailLines {
    /// Default line budget.
    pub const DEFAULT: Self = Self(120);

    /// Wrap a line count.
    #[must_use]
    pub const fn new(lines: usize) -> Self {
        Self(lines)
    }
}

impl Default for TailLines {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Deref for TailLines {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Why spawning a child failed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpawnError(String);

impl SpawnError {
    /// Wrap a reason string.
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

/// How long to wait for the host to accept connections after spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootTimeout(Duration);

impl BootTimeout {
    /// Wrap a duration.
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

/// How often to poll readiness or exit during boot/kill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollInterval(Duration);

impl PollInterval {
    /// Wrap a duration.
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

/// How often to check that a recorded child is still alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SweepInterval(Duration);

impl SweepInterval {
    /// Wrap a duration.
    #[must_use]
    pub const fn new(interval: Duration) -> Self {
        Self(interval)
    }
}

impl Deref for SweepInterval {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Time to wait after SIGTERM before SIGKILL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KillGrace(Duration);

impl KillGrace {
    /// Wrap a duration.
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

/// Timeout for a single readiness probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProbeTimeout(Duration);

impl ProbeTimeout {
    /// Wrap a duration.
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

/// Whether a child is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildStatus {
    /// Still alive.
    Running,
    /// Has exited.
    Exited,
}

/// Result of a readiness probe: the host answered the protocol handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Readiness {
    /// Host answered the handshake.
    Ready,
    /// Did not answer yet.
    NotYet,
}

/// Result of a connect-only probe: whether anything holds the port at all.
///
/// A process that answers the handshake slowly, or never, still holds the port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortListening {
    /// Something accepted the connection.
    Listening,
    /// Nothing accepted.
    Silent,
}

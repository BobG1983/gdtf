//! Results of launch and stop operations.

use crate::{
    lifecycle::{
        launch::LaunchSpec,
        orphan::OrphanPid,
        values::{BootTimeout, ChildPid, FailureTail, InstanceId, SpawnError},
    },
    link::QaPort,
};

/// Result of trying to launch a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchOutcome {
    /// Child started and became ready.
    Launched {
        /// Port it is listening on.
        port:     QaPort,
        /// Process id.
        pid:      ChildPid,
        /// Id this host records the new child under.
        instance: InstanceId,
    },
    /// Already running with a matching recipe.
    AlreadyRunning {
        /// Port in use.
        port:   QaPort,
        /// Process id.
        pid:    ChildPid,
        /// Recipe that was already launched.
        recipe: Box<LaunchSpec>,
    },
    /// Launch failed.
    Failed(LaunchFailure),
}

/// Why a launch failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchFailure {
    /// OS spawn failed.
    Spawn(SpawnError),
    /// Boot timeout without readiness.
    Timeout {
        /// Captured output tail.
        tail:   FailureTail,
        /// How long we waited.
        waited: BootTimeout,
    },
    /// Child exited before becoming ready.
    ExitedEarly(FailureTail),
    /// Running child used a different recipe.
    RecipeMismatch(Box<LaunchSpec>),
    /// Port is held by a process we did not launch.
    PortHeldByOrphan {
        /// Port in use.
        port: QaPort,
        /// Best-effort pid of the holder.
        pid:  OrphanPid,
    },
    /// This host's own records hold the port, and the search above it found none free.
    NoFreePort {
        /// Port the launch asked for.
        requested: QaPort,
    },
}

/// Result of trying to stop a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// Our managed child was stopped.
    Stopped {
        /// Process id that was stopped.
        pid: ChildPid,
    },
    /// An orphan on the port was stopped.
    OrphanStopped {
        /// Port that was freed.
        port: QaPort,
        /// Orphan pid.
        pid:  OrphanPid,
    },
    /// An orphan is still holding the port.
    OrphanHeld {
        /// Port still held.
        port: QaPort,
        /// Orphan pid.
        pid:  OrphanPid,
    },
    /// Nothing was running on that port.
    NotRunning,
}

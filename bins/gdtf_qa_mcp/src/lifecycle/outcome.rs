//! The typed results the game-lifecycle tools return (GTW-745).
//!
//! [`LaunchOutcome`] and [`StopOutcome`] are the whole answer surface of `launch_game`
//! and `stop_game`: a launch either came up, was already running (ensure-style — no
//! second spawn), or [`Failed`](LaunchOutcome::Failed) with a typed [`LaunchFailure`]
//! carrying the child's captured stderr tail; a stop either stopped the running child or
//! found nothing to stop. Nothing here panics or hangs — every path is a value.

use crate::{
    game::GamePort,
    lifecycle::values::{ChildPid, SpawnError, StderrTail},
};

/// The result of a `launch_game` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchOutcome {
    /// The game child was spawned and answered the readiness handshake.
    Launched {
        /// The loopback port the game's `net_qa` listener is bound on.
        port: GamePort,
        /// The child process's operating-system id.
        pid:  ChildPid,
    },
    /// A game child was already running — no second child was spawned (ensure-style).
    AlreadyRunning {
        /// The port the already-running child is bound on.
        port: GamePort,
        /// The already-running child's process id.
        pid:  ChildPid,
    },
    /// The launch failed; the child (if any) was reaped before this returned.
    Failed(LaunchFailure),
}

/// Why a `launch_game` failed.
///
/// Split from the successful outcome so a caller can tell "the launcher would not even
/// start" (no child, no stderr) apart from "the child started but never came up" (its
/// stderr tail is captured for diagnosis).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchFailure {
    /// The spawn call itself failed — the child never existed.
    Spawn(SpawnError),
    /// The child started but did not answer readiness before the boot timeout; the
    /// orphaned child was killed and reaped before this was returned.
    Timeout(StderrTail),
    /// The child exited on its own before it became ready.
    ExitedEarly(StderrTail),
}

/// The result of a `stop_game` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The running child was stopped and reaped.
    Stopped {
        /// The stopped child's process id.
        pid: ChildPid,
    },
    /// There was no game running to stop.
    NotRunning,
}

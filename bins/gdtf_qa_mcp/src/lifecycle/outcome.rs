//! The typed results the game-lifecycle tools return (GTW-745).
//!
//! [`LaunchOutcome`] and [`StopOutcome`] are the whole answer surface of `launch_game`
//! and `stop_game`: a launch either came up, was already running the SAME recipe
//! (ensure-style — no second spawn), or [`Failed`](LaunchOutcome::Failed) with a typed
//! [`LaunchFailure`] carrying the child's captured stderr tail; a stop either stopped the
//! running child or found nothing to stop. Nothing here panics or hangs — every path is a
//! value.
//!
//! [`AlreadyRunning`](LaunchOutcome::AlreadyRunning) carries the recipe the running child
//! was built from, and a request for a DIFFERENT recipe is
//! [`RecipeMismatch`](LaunchFailure::RecipeMismatch) rather than a success-shaped reply —
//! otherwise an agent asking for a git worktree while a main-checkout child is up would be
//! told everything is fine and would report a pass against code nobody reviewed (GTW-875).

use crate::{
    lifecycle::{
        launch::LaunchSpec,
        orphan::OrphanPid,
        values::{BootTimeout, ChildPid, SpawnError, StderrTail},
    },
    link::QaPort,
};

/// The result of a `launch_game` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchOutcome {
    /// The game child was spawned and answered the readiness handshake.
    Launched {
        /// The loopback port the game's `net_qa` listener is bound on.
        port: QaPort,
        /// The child process's operating-system id.
        pid:  ChildPid,
    },
    /// A game child built from the SAME recipe was already running — no second child was
    /// spawned (ensure-style).
    AlreadyRunning {
        /// The port the already-running child is bound on.
        port:   QaPort,
        /// The already-running child's process id.
        pid:    ChildPid,
        /// The recipe that child was launched from, so the caller can see which package,
        /// features, and checkout it is about to drive.
        recipe: Box<LaunchSpec>,
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
    ///
    /// It carries the deadline it missed as well as the tail, so the failure message can
    /// state how long the launcher actually waited instead of leaving the reader to guess
    /// which host's timeout applied (GTW-808).
    Timeout {
        /// The child's captured stderr tail.
        tail:   StderrTail,
        /// How long the launcher waited before giving up.
        waited: BootTimeout,
    },
    /// The child exited on its own before it became ready.
    ExitedEarly(StderrTail),
    /// A child is already running, but from a DIFFERENT recipe than the one requested —
    /// nothing was spawned and the running child was left alone. Carries the recipe that
    /// child was launched from so the caller can see what is actually up.
    RecipeMismatch(Box<LaunchSpec>),
    /// This manager owns no child, but the port is already held by a QA listener it never
    /// spawned — an orphan left behind by an earlier host process. Nothing was spawned:
    /// a second child would have raced the orphan for the port instead of reporting the
    /// real state (GTW-926).
    PortHeldByOrphan {
        /// The port the orphan is listening on.
        port: QaPort,
        /// The process holding it, when the operating system could name it.
        pid:  OrphanPid,
    },
}

/// The result of a `stop_game` request.
///
/// The two orphan answers exist because "this manager owns no child" and "nothing is
/// running" are different facts, and the code used to report the second when only the first
/// was established: a host replaced mid-run answers a stop about a child that is alive and
/// still holding the port. A stop now establishes who holds the port before it answers
/// (GTW-926).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The running child was stopped and reaped.
    Stopped {
        /// The stopped child's process id.
        pid: ChildPid,
    },
    /// This manager owned no child, an orphan from an earlier host process was holding the
    /// port, and it was stopped — the port is free again.
    OrphanStopped {
        /// The port the orphan was holding, now free.
        port: QaPort,
        /// The process that was holding it, when the operating system could name it.
        pid:  OrphanPid,
    },
    /// This manager owned no child and an orphan is holding the port, but it could not be
    /// stopped — either no process could be named for the port, or it survived the stop.
    OrphanHeld {
        /// The port the orphan is still holding.
        port: QaPort,
        /// The process holding it, when the operating system could name it.
        pid:  OrphanPid,
    },
    /// There was no child running to stop, and nothing is holding the port either.
    NotRunning,
}

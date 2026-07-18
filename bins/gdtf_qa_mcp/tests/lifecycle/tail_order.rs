//! WHEN a failed launch reads the child's stderr tail — after the child is reaped, never
//! before (GTW-756).
//!
//! The real [`ProcessChild`](gdtf_qa_mcp::ProcessChild) drains the child's stderr on a
//! reader thread that [`reap`](GameChild::reap) joins, so a tail read before that join can
//! catch the reader mid-drain and hand back a truncated tail — losing the diagnosis a
//! failed launch exists to carry. Against a real process that ordering only shows up when
//! the scheduler happens to be slow, which is exactly the timing this suite must not
//! depend on.
//!
//! [`ReapGatedChild`] makes it a rule instead: no process, no clock, a tail that reads
//! empty until [`reap`](GameChild::reap) has been called, and a record of every lifecycle
//! call the manager made. Both failure paths — the boot timeout and the early exit — are
//! pinned to an exact call sequence, so a manager that reads the tail one step too early
//! fails every run on every machine.

use std::{
    io,
    sync::{Arc, Mutex},
};

use gdtf_qa_mcp::{
    ChildPid, GameChild, GameLifecycle, GameManager, GamePort, GameSpawner, KillGrace,
    LaunchFailure, LaunchOutcome, StderrTail, StopOutcome, lifecycle::ChildStatus,
};

use crate::support::{fast_config, free_port};

/// The stderr text the fake child hands back once it has been reaped.
const GATED_LINE: &str = "boot-oops: the child never came up";

/// The fake child's process id — reported, never signalled (there is no process).
const GATED_PID: ChildPid = ChildPid::new(424_242);

/// A boot timeout of zero: the very first readiness miss is already past the deadline, so
/// the timeout path runs in exactly one loop pass with no waiting and no clock skew.
const NO_WAIT_BOOT_MS: u64 = 0;

/// A boot timeout the early-exit test can never reach — proving that path is taken on the
/// child's own exit, not because the deadline expired.
const UNREACHED_BOOT_MS: u64 = 2000;

/// The shared, ordered record of the lifecycle calls the manager made on the fake child.
///
/// Shared because the manager owns and drops the child on both failure paths; the test
/// reads the record afterwards.
type CallLog = Arc<Mutex<Vec<ChildCall>>>;

/// One lifecycle call the manager made on the fake child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildCall {
    /// `poll` — has the child exited on its own?
    Poll,
    /// `terminate` — the graceful SIGTERM request.
    Terminate,
    /// `wait_until_exit` — the bounded wait for the child to go after SIGTERM.
    WaitUntilExit,
    /// `kill` — the SIGKILL escalation for a child that ignored SIGTERM.
    Kill,
    /// `reap` — the wait that joins a real child's stderr reader thread.
    Reap,
    /// `stderr_tail` — the read whose order against [`Reap`](ChildCall::Reap) these tests
    /// pin.
    ReadStderrTail,
}

/// A [`GameChild`] with no process behind it, whose stderr tail is gated on being reaped.
///
/// It answers [`stderr_tail`](GameChild::stderr_tail) with an empty tail until
/// [`reap`](GameChild::reap) has been called and with [`GATED_LINE`] afterwards — the same
/// before/after the real child's reader thread has, made absolute — and records every call
/// in order. It is also a stubborn child: it never reports an exit, so a stop must
/// escalate SIGTERM to SIGKILL, putting the whole orphan-kill path in the recorded
/// sequence.
struct ReapGatedChild {
    /// The stderr text, readable only once the child has been reaped.
    tail:   StderrTail,
    /// What the child reports when polled — whether it exited on its own.
    status: ChildStatus,
    /// The shared record of calls, in the order they were made.
    calls:  CallLog,
}

impl ReapGatedChild {
    /// Append one call to the shared record.
    fn record(&self, call: ChildCall) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(call);
        }
    }

    /// Whether [`reap`](GameChild::reap) has already been called — what gates the tail.
    fn reaped(&self) -> bool {
        self.calls
            .lock()
            .is_ok_and(|calls| calls.contains(&ChildCall::Reap))
    }
}

impl GameChild for ReapGatedChild {
    fn pid(&self) -> ChildPid {
        GATED_PID
    }

    fn poll(&mut self) -> ChildStatus {
        self.record(ChildCall::Poll);
        self.status
    }

    fn terminate(&mut self) {
        self.record(ChildCall::Terminate);
    }

    fn kill(&mut self) {
        self.record(ChildCall::Kill);
    }

    fn wait_until_exit(&mut self, _within: KillGrace) -> ChildStatus {
        self.record(ChildCall::WaitUntilExit);
        ChildStatus::Running
    }

    fn reap(&mut self) {
        self.record(ChildCall::Reap);
    }

    fn stderr_tail(&self) -> StderrTail {
        self.record(ChildCall::ReadStderrTail);
        if self.reaped() {
            self.tail.clone()
        } else {
            StderrTail::default()
        }
    }
}

/// Hands the manager one [`ReapGatedChild`] — no process is launched at all.
struct ReapGatedSpawner {
    /// What the child reports when polled.
    status: ChildStatus,
    /// The record the spawned child writes its calls into.
    calls:  CallLog,
}

impl GameSpawner for ReapGatedSpawner {
    fn spawn(&self, _port: GamePort) -> io::Result<Box<dyn GameChild>> {
        Ok(Box::new(ReapGatedChild {
            tail:   StderrTail::new(GATED_LINE.to_owned()),
            status: self.status,
            calls:  Arc::clone(&self.calls),
        }))
    }
}

/// Build a manager over a fake child that reports `status` when polled, together with the
/// call record that child writes into.
fn manager_over_gated_child(status: ChildStatus, boot_ms: u64) -> (GameManager, CallLog) {
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let spawner = ReapGatedSpawner {
        status,
        calls: Arc::clone(&calls),
    };
    let manager = GameManager::with_config(Box::new(spawner), fast_config(boot_ms));
    (manager, calls)
}

/// Read the call record back after the manager has dropped the child.
fn recorded(calls: &CallLog) -> Vec<ChildCall> {
    let Ok(log) = calls.lock() else {
        unreachable!("the fake child's call record is not poisoned");
    };
    log.clone()
}

/// A launch that times out kills and reaps the orphan BEFORE it reads the stderr tail, so
/// the failure carries the child's last words rather than whatever had been drained so far.
#[test]
fn timeout_reads_the_stderr_tail_after_reaping_the_orphan() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Running, NO_WAIT_BOOT_MS);

    let outcome = manager.launch(GamePort::new(free_port()));
    let LaunchOutcome::Failed(LaunchFailure::Timeout(tail)) = outcome else {
        unreachable!("no readiness endpoint means the launch times out: {outcome:?}");
    };
    assert_eq!(
        tail.as_str(),
        GATED_LINE,
        "the timeout failure carries the tail the child only yields once reaped",
    );
    assert_eq!(
        recorded(&calls),
        vec![
            ChildCall::Poll,
            ChildCall::Terminate,
            ChildCall::WaitUntilExit,
            ChildCall::Kill,
            ChildCall::Reap,
            ChildCall::ReadStderrTail,
        ],
        "the orphan is stopped and reaped, and only then is its stderr tail read",
    );
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

/// A child that exits on its own is reaped BEFORE its stderr tail is read, for the same
/// reason — reaping is what finishes draining the closed pipe.
#[test]
fn early_exit_reads_the_stderr_tail_after_reaping_the_child() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Exited, UNREACHED_BOOT_MS);

    let outcome = manager.launch(GamePort::new(free_port()));
    let LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail)) = outcome else {
        unreachable!("a child that has exited fails the launch as an early exit: {outcome:?}");
    };
    assert_eq!(
        tail.as_str(),
        GATED_LINE,
        "the early-exit failure carries the tail the child only yields once reaped",
    );
    assert_eq!(
        recorded(&calls),
        vec![ChildCall::Poll, ChildCall::Reap, ChildCall::ReadStderrTail],
        "the exited child is reaped, and only then is its stderr tail read",
    );
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

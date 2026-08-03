use std::{
    io,
    sync::{Arc, Mutex},
};

use gdtf_qa_mcp::{
    ChildPid, ChildSpawner, FailureTail, HostLifecycle, HostManager, KillGrace, LaunchFailure,
    LaunchOutcome, LaunchSpec, ManagedChild, OutputTail, QaPort, StopOutcome, TailLines,
    lifecycle::ChildStatus,
};

use crate::support::{fast_config, free_port};

const GATED_LINE: &str = "boot-oops: the child never came up";

const GATED_PID: ChildPid = ChildPid::new(424_242);

const NO_WAIT_BOOT_MS: u64 = 0;

const UNREACHED_BOOT_MS: u64 = 2000;

type CallLog = Arc<Mutex<Vec<ChildCall>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildCall {
        Poll,
        Terminate,
        WaitUntilExit,
        Kill,
        Reap,
            ReadFailureTail,
}

struct ReapGatedChild {
        tail:   FailureTail,
        status: ChildStatus,
        calls:  CallLog,
}

impl ReapGatedChild {
        fn record(&self, call: ChildCall) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(call);
        }
    }

        fn reaped(&self) -> bool {
        self.calls
            .lock()
            .is_ok_and(|calls| calls.contains(&ChildCall::Reap))
    }
}

impl ManagedChild for ReapGatedChild {
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

    fn failure_tail(&self) -> FailureTail {
        self.record(ChildCall::ReadFailureTail);
        if self.reaped() {
            self.tail.clone()
        } else {
            FailureTail::default()
        }
    }

    fn output_tail(&self, _max: TailLines) -> OutputTail {
        OutputTail::new((*self.failure_tail()).clone())
    }
}

struct ReapGatedSpawner {
        status: ChildStatus,
        calls:  CallLog,
}

impl ChildSpawner for ReapGatedSpawner {
    fn spawn(&self, _port: QaPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        Ok(Box::new(ReapGatedChild {
            tail:   FailureTail::new(GATED_LINE.to_owned()),
            status: self.status,
            calls:  Arc::clone(&self.calls),
        }))
    }
}

fn manager_over_gated_child(status: ChildStatus, boot_ms: u64) -> (HostManager, CallLog) {
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let spawner = ReapGatedSpawner {
        status,
        calls: Arc::clone(&calls),
    };
    let manager = HostManager::with_config(Box::new(spawner), fast_config(boot_ms));
    (manager, calls)
}

fn recorded(calls: &CallLog) -> Vec<ChildCall> {
    let Ok(log) = calls.lock() else {
        unreachable!("the fake child's call record is not poisoned");
    };
    log.clone()
}

#[test]
fn timeout_reads_the_output_tail_after_reaping_the_orphan() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Running, NO_WAIT_BOOT_MS);

    let outcome = manager.launch(QaPort::new(free_port()), &LaunchSpec::game_default());
    let LaunchOutcome::Failed(LaunchFailure::Timeout { tail, .. }) = outcome else {
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
            ChildCall::ReadFailureTail,
        ],
        "the orphan is stopped and reaped, and only then is its output tail read",
    );
    assert_eq!(
        manager.stop(QaPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

#[test]
fn early_exit_reads_the_output_tail_after_reaping_the_child() {
    let (mut manager, calls) = manager_over_gated_child(ChildStatus::Exited, UNREACHED_BOOT_MS);

    let outcome = manager.launch(QaPort::new(free_port()), &LaunchSpec::game_default());
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
        vec![ChildCall::Poll, ChildCall::Reap, ChildCall::ReadFailureTail],
        "the exited child is reaped, and only then is its output tail read",
    );
    assert_eq!(
        manager.stop(QaPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

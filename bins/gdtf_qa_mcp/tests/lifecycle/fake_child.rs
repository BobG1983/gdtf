use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use gdtf_qa_mcp::{
    ChildPid, ChildSpawner, FailureTail, KillGrace, LaunchSpec, ManagedChild, OutputTail, QaPort,
    TailLines, lifecycle::ChildStatus,
};

use crate::support::FakeGameGate;

pub(crate) const GATED_LINE: &str = "boot-oops: the child never came up";

const FIRST_GATED_PID: u32 = 424_242;

pub(crate) type CallLog = Arc<Mutex<Vec<ChildCall>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChildCall {
    Poll,
    Terminate,
    WaitUntilExit,
    Kill,
    Reap,
    ReadFailureTail,
}

pub(crate) struct ReapGatedChild {
    pid:    ChildPid,
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
        self.pid
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

pub(crate) struct ReapGatedSpawner {
    status:   ChildStatus,
    calls:    CallLog,
    gate:     Option<FakeGameGate>,
    next_pid: AtomicU32,
}

impl ReapGatedSpawner {
    pub(crate) const fn new(status: ChildStatus, calls: CallLog) -> Self {
        Self {
            status,
            calls,
            gate: None,
            next_pid: AtomicU32::new(FIRST_GATED_PID),
        }
    }

    pub(crate) const fn gated(status: ChildStatus, calls: CallLog, gate: FakeGameGate) -> Self {
        Self {
            status,
            calls,
            gate: Some(gate),
            next_pid: AtomicU32::new(FIRST_GATED_PID),
        }
    }
}

impl ChildSpawner for ReapGatedSpawner {
    fn spawn(&self, _port: QaPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        let child = ReapGatedChild {
            pid:    ChildPid::new(self.next_pid.fetch_add(1, Ordering::SeqCst)),
            tail:   FailureTail::new(GATED_LINE.to_owned()),
            status: self.status,
            calls:  Arc::clone(&self.calls),
        };
        if let Some(gate) = self.gate.as_ref() {
            gate.open();
        }
        Ok(Box::new(child))
    }
}

pub(crate) fn recorded(calls: &CallLog) -> Vec<ChildCall> {
    let Ok(log) = calls.lock() else {
        unreachable!("the fake child's call record is not poisoned");
    };
    log.clone()
}

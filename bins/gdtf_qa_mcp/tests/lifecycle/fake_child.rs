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

const FIRST_PORT_GATED_PID: u32 = 515_151;

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
    output: OutputTail,
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
        self.output.clone()
    }
}

pub(crate) struct ReapGatedSpawner {
    status:   ChildStatus,
    calls:    CallLog,
    next_pid: AtomicU32,
}

impl ReapGatedSpawner {
    pub(crate) const fn new(status: ChildStatus, calls: CallLog) -> Self {
        Self {
            status,
            calls,
            next_pid: AtomicU32::new(FIRST_GATED_PID),
        }
    }
}

impl ChildSpawner for ReapGatedSpawner {
    fn spawn(&self, _port: QaPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        Ok(Box::new(ReapGatedChild {
            pid:    ChildPid::new(self.next_pid.fetch_add(1, Ordering::SeqCst)),
            tail:   FailureTail::new(GATED_LINE.to_owned()),
            output: OutputTail::new(GATED_LINE.to_owned()),
            status: self.status,
            calls:  Arc::clone(&self.calls),
        }))
    }
}

/// Tail one child prints, naming the port that child was handed.
pub(crate) fn tail_on(port: QaPort) -> String {
    format!("fake child listening on port {}", *port)
}

/// Spawner that opens the gate belonging to the port it is handed.
pub(crate) struct PortGatedSpawner {
    calls:    CallLog,
    gates:    Vec<(QaPort, FakeGameGate)>,
    next_pid: AtomicU32,
}

impl PortGatedSpawner {
    pub(crate) const fn new(calls: CallLog, gates: Vec<(QaPort, FakeGameGate)>) -> Self {
        Self {
            calls,
            gates,
            next_pid: AtomicU32::new(FIRST_PORT_GATED_PID),
        }
    }
}

impl ChildSpawner for PortGatedSpawner {
    fn spawn(&self, port: QaPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        for (gated, gate) in &self.gates {
            if *gated == port {
                gate.open();
            }
        }
        Ok(Box::new(ReapGatedChild {
            pid:    ChildPid::new(self.next_pid.fetch_add(1, Ordering::SeqCst)),
            tail:   FailureTail::new(GATED_LINE.to_owned()),
            output: OutputTail::new(tail_on(port)),
            status: ChildStatus::Running,
            calls:  Arc::clone(&self.calls),
        }))
    }
}

pub(crate) fn counted(calls: &CallLog, call: ChildCall) -> usize {
    recorded(calls).iter().filter(|made| **made == call).count()
}

pub(crate) fn recorded(calls: &CallLog) -> Vec<ChildCall> {
    let Ok(log) = calls.lock() else {
        unreachable!("the fake child's call record is not poisoned");
    };
    log.clone()
}

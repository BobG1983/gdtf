use crate::{
    lifecycle::{
        launch::LaunchSpec,
        orphan::OrphanPid,
        values::{BootTimeout, ChildPid, FailureTail, SpawnError},
    },
    link::QaPort,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchOutcome {
        Launched {
                port: QaPort,
                pid:  ChildPid,
    },
            AlreadyRunning {
                port:   QaPort,
                pid:    ChildPid,
                        recipe: Box<LaunchSpec>,
    },
        Failed(LaunchFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchFailure {
        Spawn(SpawnError),
                            Timeout {
                tail:   FailureTail,
                waited: BootTimeout,
    },
        ExitedEarly(FailureTail),
                RecipeMismatch(Box<LaunchSpec>),
                    PortHeldByOrphan {
                port: QaPort,
                pid:  OrphanPid,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
        Stopped {
                pid: ChildPid,
    },
            OrphanStopped {
                port: QaPort,
                pid:  OrphanPid,
    },
            OrphanHeld {
                port: QaPort,
                pid:  OrphanPid,
    },
        NotRunning,
}

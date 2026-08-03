use core::{ops::Deref, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChildPid(u32);

impl ChildPid {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FailureTail(String);

impl FailureTail {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OutputTail(String);

impl OutputTail {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TailLines(usize);

impl TailLines {
        pub const DEFAULT: Self = Self(120);

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpawnError(String);

impl SpawnError {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootTimeout(Duration);

impl BootTimeout {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollInterval(Duration);

impl PollInterval {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KillGrace(Duration);

impl KillGrace {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProbeTimeout(Duration);

impl ProbeTimeout {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildStatus {
        Running,
        Exited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Readiness {
        Ready,
        NotYet,
}

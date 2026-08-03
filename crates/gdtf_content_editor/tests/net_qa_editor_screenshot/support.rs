use std::error::Error;

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

pub(crate) const TEST_SETTLE: u32 = 5;

pub(crate) const TEST_POLL_BUDGET: u32 = 240;

pub(crate) const EDITING_UPDATES: u32 = 10_000;

pub(crate) const DRIVE_UPDATES: u32 = 2_000;

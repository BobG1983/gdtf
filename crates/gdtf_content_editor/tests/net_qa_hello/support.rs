use std::error::Error;

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

pub(crate) const EDITING_EXCHANGES: usize = 3;

pub(crate) const EDITING_UPDATES: u32 = 10_000;

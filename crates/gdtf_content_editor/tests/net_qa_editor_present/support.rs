use std::error::Error;

pub(crate) type TestError = Box<dyn Error>;

pub(crate) type TestResult = Result<(), TestError>;

//! The shared aliases this suite reports failures through (GTW-918).

use std::error::Error;

/// The boxed error every fallible step in this suite reports through.
pub(crate) type TestError = Box<dyn Error>;

/// The return type of a test that can fail on setup rather than on an assertion.
pub(crate) type TestResult = Result<(), TestError>;

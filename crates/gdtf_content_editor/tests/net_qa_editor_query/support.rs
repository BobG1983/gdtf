//! The suite's shared aliases, loop caps and fixture constants (GTW-805).

use std::{error::Error, time::Duration};

use gdtf_qa_protocol::envelope::QaResponse;

/// A boxed error so a test's `?` spans `io::Error`, the codec's error type and a bare
/// message. `Send + Sync` because the client half's result crosses a thread boundary.
pub(crate) type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
pub(crate) type TestResult = Result<(), TestError>;

/// One phase's replies, in order — or the failure that ended the client half.
pub(crate) type PhaseReport = Result<Vec<QaResponse>, TestError>;

/// A SAFETY NET on the frame loop, not a timing budget: the loop exits as soon as both
/// phases have reported.
pub(crate) const MAX_UPDATES: u32 = 800;

/// How long one frame iteration waits on the client thread before driving another update.
pub(crate) const POLL_STEP: Duration = Duration::from_millis(10);

/// A SAFETY NET on the client's readiness poll.
pub(crate) const MAX_READINESS_POLLS: u32 = 400;

/// The display name written into the TERRAIN draft, read back off the draft topic.
pub(crate) const DRAFT_NAME: &str = "Rusted Barricade";

/// The dangling key recorded into the content-integrity report, read back off the
/// validation topic.
pub(crate) const DANGLING_KEY: &str = "not-a-real-terrain";

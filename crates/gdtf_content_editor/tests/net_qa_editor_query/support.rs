//! The suite's shared aliases and loop caps (GTW-805; re-homed onto the real editor in
//! GTW-879).

use std::{error::Error, time::Duration};

use gdtf_qa_protocol::envelope::QaResponse;

/// A boxed error so a test's `?` spans `io::Error`, the codec's error type and a bare
/// message. `Send + Sync` because the client half's result crosses a thread boundary.
pub(crate) type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
pub(crate) type TestResult = Result<(), TestError>;

/// One phase's replies, in order — or the failure that ended the client half.
pub(crate) type PhaseReport = Result<Vec<QaResponse>, TestError>;

/// How many replies the `Load`-phase window collects.
///
/// The ONE place the client's exchange list and the assertions' destructuring agree, and they
/// agree at COMPILE time: `client::drive` builds a `[QaResponse; LOAD_PHASE_REPLIES]` array
/// literal and `assertions::assert_load_phase` destructures the same fixed-size array, so an
/// exchange added to the wire without an assertion arm fails to build. GTW-879 landed with a
/// `Vec` on one side and a five-element slice pattern on the other: the sixth exchange made
/// the pattern's fallback arm fire on every run, and every assertion below it — both phases'
/// — was dead code that had never executed.
pub(crate) const LOAD_PHASE_REPLIES: usize = 6;

/// How long the test body waits for the client to connect and put its first request on the
/// wire. The listener's accept loop runs on its own thread, so this wait needs no frames —
/// which is the point: no frame runs before the first request is pending, so the reply is
/// answered while the editor's asset pass has barely started.
pub(crate) const OPEN_WAIT: Duration = Duration::from_secs(10);

/// A SAFETY NET on the frame loop that waits for ONE phase report, not a timing budget: the
/// loop exits as soon as that phase reports, and each iteration drives exactly one frame.
pub(crate) const PHASE_UPDATES: u32 = 2_000;

/// How long one frame iteration waits on the client thread before driving another update.
pub(crate) const POLL_STEP: Duration = Duration::from_millis(10);

/// A SAFETY NET on the frames the editor's own `Load` asset pass may take — the same
/// generous cap `tests/prefab_mode/harness.rs` uses, because an async folder load under
/// parallel `cargo` contention takes a non-deterministic number of frames.
pub(crate) const EDITING_UPDATES: u32 = 10_000;

/// A SAFETY NET on the client's readiness poll. Each poll is one socket round trip answered
/// by one app frame, so this must comfortably exceed [`EDITING_UPDATES`] — the client polls
/// once per frame the editor spends resolving its registries.
pub(crate) const MAX_READINESS_POLLS: u32 = 20_000;

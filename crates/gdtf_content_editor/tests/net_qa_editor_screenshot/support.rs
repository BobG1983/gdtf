//! The suite's shared aliases, loop caps and injected tunables (GTW-880).

use std::error::Error;

/// A boxed error so a test's `?` spans `io::Error`, the codec's error type and a bare
/// message. `Send + Sync` because the client half's result crosses a thread boundary.
pub(crate) type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
pub(crate) type TestResult = Result<(), TestError>;

/// The settle window both tests pin, in frames.
///
/// Short so a test does not spin the default egui-calibrated 30, but non-zero so the
/// settle-then-capture ordering is observable: the capture must not be spawned until this
/// many frames after the claim.
pub(crate) const TEST_SETTLE: u32 = 5;

/// The poll budget both tests pin, in frames.
///
/// Generous enough for a real GPU readback to flush the PNG in the GPU test, small enough
/// that the no-renderer test reaches its typed timeout quickly.
pub(crate) const TEST_POLL_BUDGET: u32 = 240;

/// A SAFETY NET on the frames the editor's own `Load` asset pass may take — the same
/// generous cap `tests/net_qa_editor_query/support.rs` uses.
pub(crate) const EDITING_UPDATES: u32 = 10_000;

/// A SAFETY NET on the frame loop that waits for the capture to be spawned, or for the reply
/// to arrive. Not a timing budget: each loop exits as soon as its condition holds.
pub(crate) const DRIVE_UPDATES: u32 = 2_000;

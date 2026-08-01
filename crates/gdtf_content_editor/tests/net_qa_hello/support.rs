//! The suite's shared aliases and loop caps (GTW-804; split out of the flat file in GTW-896).

use std::{error::Error, time::Duration};

use gdtf_content_editor::EditorState;
use gdtf_qa_protocol::message::QaResponse;

/// A boxed error so a test's `?` spans `io::Error`, the codec's error type and a bare
/// message. `Send + Sync` because the client half's result crosses a thread boundary back to
/// the test body over an [`std::sync::mpsc`] channel.
pub(crate) type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
pub(crate) type TestResult = Result<(), TestError>;

/// The `Editing` client half's batched result — every reply it collected, in order.
pub(crate) type ClientResult = Result<Vec<QaResponse>, TestError>;

/// ONE reply as the `Load` client half reports it, or the failure that ended it.
pub(crate) type ReplyReport = Result<QaResponse, TestError>;

/// One reply paired with the editor state of the frame that produced it.
///
/// The pairing is EXACT rather than approximate: Bevy applies a queued state transition in the
/// `StateTransition` schedule, which runs before `Update` in the same frame, so the state read
/// after frame F is the state the request drain saw during frame F.
pub(crate) type TaggedReply = (QaResponse, Option<EditorState>);

/// How many replies the `Editing`-side case collects over its one connection: the handshake, the
/// mismatched-version refusal, and the unsupported request.
pub(crate) const EDITING_EXCHANGES: usize = 3;

/// A SAFETY NET on the frame loop, not a timing budget: the loop exits as soon as the client
/// thread has reported, and each iteration waits up to [`POLL_STEP`] for it.
pub(crate) const MAX_UPDATES: u32 = 400;

/// A SAFETY NET on the frames the editor's own `Load` asset pass may take — the same generous
/// cap `tests/prefab_mode/harness.rs` uses, because an async folder load under parallel
/// `cargo` contention takes a non-deterministic number of frames.
pub(crate) const EDITING_UPDATES: u32 = 10_000;

/// How long one frame iteration waits on the client thread before driving another update.
pub(crate) const POLL_STEP: Duration = Duration::from_millis(10);

/// How long a `Load`-phase test body waits for the client to connect and put its request on the
/// wire. The listener's accept loop runs on its own thread, so this wait costs the app no
/// frames — which is the point: no frame runs before the request is pending, so it is answered
/// while the editor's asset pass has barely started.
pub(crate) const OPEN_WAIT: Duration = Duration::from_secs(10);

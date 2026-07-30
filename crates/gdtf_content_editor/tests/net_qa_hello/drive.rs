//! Driving the real app's frames while the client half talks to it (GTW-804; the state-tagging
//! collector added in GTW-896).
//!
//! The replies only exist once the plugin's drain has run in `Update`, so the app must be ticked
//! while the client waits — and each iteration waits [`POLL_STEP`] for the next report before
//! ticking again, which paces the drive to the client rather than to a frame budget.

use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, RecvTimeoutError},
};

use bevy::prelude::*;
use gdtf_qa_protocol::envelope::QaResponse;

use crate::{
    harness::editor_state,
    support::{
        ClientResult, EDITING_EXCHANGES, MAX_UPDATES, POLL_STEP, ReplyReport, TaggedReply,
        TestError,
    },
};

/// Turn a collected reply list into the fixed-size array the assertions destructure.
///
/// A short read reports as a client-half failure rather than as a panic inside the assertions.
fn exactly_all<T: Debug, const N: usize>(replies: Vec<T>) -> Result<[T; N], TestError> {
    replies.try_into().map_err(|replies: Vec<T>| {
        TestError::from(format!("expected {N} replies, got {replies:?}"))
    })
}

/// Drive one frame per iteration until the client half reports its whole batch.
///
/// # Errors
///
/// The client half's own failure, a batch of the wrong length, or a frame budget spent without
/// the client reporting at all.
pub(crate) fn drive_until_batched(
    app: &mut App,
    rx: &Receiver<ClientResult>,
) -> Result<[QaResponse; EDITING_EXCHANGES], TestError> {
    let mut reported = None;
    for _ in 0..MAX_UPDATES {
        app.update();
        match rx.recv_timeout(POLL_STEP) {
            Ok(result) => {
                reported = Some(result);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let replies = reported.ok_or("the client thread never reported an exchange")??;
    exactly_all(replies)
}

/// Drive one frame per iteration until the client reports its ONE reply, paired with the editor
/// state of the frame that answered it (GTW-896).
///
/// The pairing is exact, not a guess: Bevy applies a queued state transition in the
/// `StateTransition` schedule, which runs before `Update` in the same frame, so the state read
/// right after frame F is the state the request drain ran under during frame F. And the reply is
/// read the same iteration it is produced — the loop waits [`POLL_STEP`] on the client after
/// every single frame — so the tag is the answering frame's state, not a later frame's.
///
/// # Errors
///
/// The client half's own failure, or a frame budget spent before the reply arrived.
pub(crate) fn drive_until_tagged(
    app: &mut App,
    rx: &Receiver<ReplyReport>,
) -> Result<TaggedReply, TestError> {
    for _ in 0..MAX_UPDATES {
        app.update();
        // The state the frame just driven ran under — read BEFORE another frame can move it.
        let during = editor_state(app);
        match rx.recv_timeout(POLL_STEP) {
            Ok(report) => return Ok((report?, during)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Err("the client thread never reported its reply".into())
}

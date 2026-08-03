use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, RecvTimeoutError},
};

use bevy::prelude::*;
use gdtf_qa_protocol::message::QaResponse;

use crate::{
    harness::editor_state,
    support::{
        ClientResult, EDITING_EXCHANGES, MAX_UPDATES, POLL_STEP, ReplyReport, TaggedReply,
        TestError,
    },
};

fn exactly_all<T: Debug, const N: usize>(replies: Vec<T>) -> Result<[T; N], TestError> {
    replies.try_into().map_err(|replies: Vec<T>| {
        TestError::from(format!("expected {N} replies, got {replies:?}"))
    })
}

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

pub(crate) fn drive_until_tagged(
    app: &mut App,
    rx: &Receiver<ReplyReport>,
) -> Result<TaggedReply, TestError> {
    for _ in 0..MAX_UPDATES {
        app.update();
        let during = editor_state(app);
        match rx.recv_timeout(POLL_STEP) {
            Ok(report) => return Ok((report?, during)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Err("the client thread never reported its reply".into())
}

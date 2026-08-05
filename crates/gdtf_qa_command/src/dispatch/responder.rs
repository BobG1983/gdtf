//! Typed wrapper around a transport responder for one command.

use core::marker::PhantomData;

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{CommandOutcome, CommandReplyRon, RefusalNote, ReplyAttachment, UnavailableCode},
    message::{QaError, QaResponse},
};

use crate::command::QaCommand;

/// Responder that serializes `C::Reply` into a protocol outcome.
pub struct CommandResponder<C: QaCommand> {
    inner:   Responder,
    command: PhantomData<fn() -> C>,
}

impl<C: QaCommand> CommandResponder<C> {
    /// Wrap a transport responder.
    #[must_use]
    pub const fn new(inner: Responder) -> Self {
        Self {
            inner,
            command: PhantomData,
        }
    }

    /// Serialize `reply` and send a successful outcome.
    pub fn answer(self, reply: &C::Reply) {
        self.answer_with(reply, Vec::new());
    }

    /// Serialize `reply` with attachments and send a successful outcome.
    pub fn answer_with(self, reply: &C::Reply, attachments: Vec<ReplyAttachment>) {
        match ron::ser::to_string(reply) {
            Ok(text) => self.inner.reply(QaResponse::Outcome(CommandOutcome::Ran {
                reply: CommandReplyRon::new(text),
                attachments,
            })),
            Err(fault) => {
                error!(
                    command = C::NAME.as_str(),
                    %fault,
                    "net_qa: a command's declared reply would not serialise"
                );
                self.inner.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }

    /// Send an unavailable outcome.
    pub fn unavailable(self, code: UnavailableCode, note: RefusalNote) {
        self.inner
            .reply(QaResponse::Outcome(CommandOutcome::Unavailable {
                code,
                note,
            }));
    }

    /// Unwrap to the raw transport responder for a hand-built protocol answer.
    #[must_use]
    pub fn into_inner(self) -> Responder {
        self.inner
    }
}

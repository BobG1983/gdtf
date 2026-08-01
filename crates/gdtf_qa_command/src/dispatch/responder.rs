//! [`CommandResponder`] — the one-shot reply channel typed to one command's reply.

use core::marker::PhantomData;

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{CommandOutcome, CommandReplyJson, RefusalNote, ReplyAttachment, UnavailableCode},
    message::{QaError, QaResponse},
};

use crate::command::QaCommand;

/// The one-shot reply channel for one call, typed to its command's declared reply.
///
/// A handler never sees a raw [`Responder`] and never sees JSON: it gets `C::Args` in and
/// answers `&C::Reply` out, so the shape a client was promised in the catalogue is the only
/// shape the handler can produce.
///
/// `PhantomData<fn() -> C>` rather than `PhantomData<C>` keeps this `Send + Sync` whatever
/// `C` is, and stops the marker implying ownership of a `C`.
pub struct CommandResponder<C: QaCommand> {
    /// The channel back to the socket.
    inner:   Responder,
    /// Which command's reply type this responder accepts.
    command: PhantomData<fn() -> C>,
}

impl<C: QaCommand> CommandResponder<C> {
    /// Type a raw transport responder to this command's reply.
    #[must_use]
    pub const fn new(inner: Responder) -> Self {
        Self {
            inner,
            command: PhantomData,
        }
    }

    /// Answer with the command's declared reply.
    pub fn answer(self, reply: &C::Reply) {
        self.answer_with(reply, Vec::new());
    }

    /// Answer with the declared reply plus files the caller should receive.
    ///
    /// An encode failure cannot produce the declared reply and must not panic, so it is
    /// answered [`Malformed`](QaError::Malformed) — the frame the host would have written
    /// is not a decodable reply — and logged at `error` so it is visible in the host's own
    /// output rather than only on the wire.
    pub fn answer_with(self, reply: &C::Reply, attachments: Vec<ReplyAttachment>) {
        match serde_json::to_string(reply) {
            Ok(json) => self.inner.reply(QaResponse::Outcome(CommandOutcome::Ran {
                reply: CommandReplyJson::new(json),
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

    /// Answer that a precondition only the handler can see is missing.
    ///
    /// Distinct from the route-time refusal [`admit()`](super::admit()) produces: that one is a
    /// pure function of the frame's facts, this one is for a precondition the handler
    /// discovers while doing the work.
    pub fn unavailable(self, code: UnavailableCode, note: RefusalNote) {
        self.inner
            .reply(QaResponse::Outcome(CommandOutcome::Unavailable {
                code,
                note,
            }));
    }

    /// The untyped channel underneath — the deadline sweep's only way to answer a
    /// [`QaError`], which is not a `C::Reply` and never can be.
    pub(crate) fn into_inner(self) -> Responder {
        self.inner
    }
}

//! Typed wrapper around a transport responder for one command.

use core::marker::PhantomData;

use bevy::prelude::*;
use cobalt_mcp_protocol::{
    command::{
        ArgSchemaRon, ArgumentFault, CommandOutcome, CommandReplyRon, RefusalNote, ReplyAttachment,
        UnavailableCode, shape_text,
    },
    message::{McpResponse, McpSessionError},
};

use crate::{command::McpCommand, transport::Responder};

/// Responder that serializes `C::Reply` into a protocol outcome.
pub struct CommandResponder<C: McpCommand> {
    inner:   Responder,
    command: PhantomData<fn() -> C>,
}

impl<C: McpCommand> CommandResponder<C> {
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
            Ok(text) => self.inner.reply(McpResponse::Outcome(CommandOutcome::Ran {
                reply: CommandReplyRon::new(text),
                attachments,
            })),
            Err(fault) => {
                error!(
                    command = C::NAME.as_str(),
                    %fault,
                    "mcp: a command's declared reply would not serialise"
                );
                self.inner
                    .reply(McpResponse::Error(McpSessionError::Malformed));
            }
        }
    }

    /// Send a bad-arguments outcome the handler discovered, with the command's own shape.
    pub fn bad_arguments(self, detail: ArgumentFault) {
        self.inner
            .reply(McpResponse::Outcome(CommandOutcome::BadArguments {
                detail,
                schema: ArgSchemaRon::new(shape_text::<C::Args>()),
            }));
    }

    /// Send an unavailable outcome.
    pub fn unavailable(self, code: UnavailableCode, note: RefusalNote) {
        self.inner
            .reply(McpResponse::Outcome(CommandOutcome::Unavailable {
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

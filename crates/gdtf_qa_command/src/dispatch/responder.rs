use core::marker::PhantomData;

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{CommandOutcome, CommandReplyJson, RefusalNote, ReplyAttachment, UnavailableCode},
    message::{QaError, QaResponse},
};

use crate::command::QaCommand;

pub struct CommandResponder<C: QaCommand> {
        inner:   Responder,
        command: PhantomData<fn() -> C>,
}

impl<C: QaCommand> CommandResponder<C> {
        #[must_use]
    pub const fn new(inner: Responder) -> Self {
        Self {
            inner,
            command: PhantomData,
        }
    }

        pub fn answer(self, reply: &C::Reply) {
        self.answer_with(reply, Vec::new());
    }

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

                        pub fn unavailable(self, code: UnavailableCode, note: RefusalNote) {
        self.inner
            .reply(QaResponse::Outcome(CommandOutcome::Unavailable {
                code,
                note,
            }));
    }

            pub(crate) fn into_inner(self) -> Responder {
        self.inner
    }
}

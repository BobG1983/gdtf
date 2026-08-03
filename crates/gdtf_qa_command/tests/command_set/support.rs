use std::sync::mpsc::{Receiver, TryRecvError};

use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandOutcome, RunOptions},
    message::QaResponse,
};

pub(crate) fn args(json: &str) -> CommandArgsJson {
    CommandArgsJson::new(json.to_owned())
}

pub(crate) fn plain() -> RunOptions {
    RunOptions::default()
}

pub(crate) fn answer(channel: &Receiver<QaResponse>) -> QaResponse {
    match channel.try_recv() {
        Ok(response) => response,
        Err(TryRecvError::Empty) => unreachable!("no answer arrived on the call's channel"),
        Err(TryRecvError::Disconnected) => {
            unreachable!("the call's responder was dropped without answering")
        }
    }
}

pub(crate) fn no_answer_yet(channel: &Receiver<QaResponse>) {
    assert_eq!(
        channel.try_recv().err(),
        Some(TryRecvError::Empty),
        "the call must not have been answered yet"
    );
}

pub(crate) fn outcome(channel: &Receiver<QaResponse>) -> CommandOutcome {
    match answer(channel) {
        QaResponse::Outcome(outcome) => outcome,
        other => unreachable!("expected a command outcome, got {other:?}"),
    }
}

pub(crate) fn ran<T: serde::de::DeserializeOwned>(channel: &Receiver<QaResponse>) -> T {
    let outcome = outcome(channel);
    let CommandOutcome::Ran { reply, attachments } = outcome else {
        unreachable!("expected the command to run, got {outcome:?}");
    };
    assert!(
        attachments.is_empty(),
        "no fake command attaches a file: {attachments:?}"
    );
    match serde_json::from_str::<T>(reply.as_str()) {
        Ok(value) => value,
        Err(fault) => unreachable!(
            "the reply did not decode into the command's declared type ({fault}): {}",
            reply.as_str()
        ),
    }
}

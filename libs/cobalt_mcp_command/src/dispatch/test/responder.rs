use cobalt_mcp_protocol::{
    command::{
        ArgumentFault, ArtifactPath, AttachmentKind, CommandOutcome, RefusalNote, ReplyAttachment,
        UnavailableCode, shape_text,
    },
    message::QaResponse,
};
use cobalt_mcp_transport::Responder;

use crate::{
    command::QaCommand,
    dispatch::CommandResponder,
    test_support::{FakeLevel, FakePhase, FakePhaseReply, FakeReady},
};

fn a_reply() -> FakePhaseReply {
    FakePhaseReply {
        ready: FakeReady::new(true),
        level: FakeLevel::new(3),
    }
}

fn an_attachment() -> ReplyAttachment {
    ReplyAttachment::new(
        AttachmentKind::Png,
        ArtifactPath::new("fake/shot.png".to_owned()),
    )
}

#[test]
fn answer_produces_a_ran_outcome_with_no_attachments() {
    let (raw, channel) = Responder::channel();
    CommandResponder::<FakePhase>::new(raw).answer(&a_reply());

    let Ok(QaResponse::Outcome(CommandOutcome::Ran { reply, attachments })) = channel.try_recv()
    else {
        unreachable!("answer must produce a Ran outcome");
    };
    assert!(attachments.is_empty(), "answer attaches nothing");
    assert_eq!(
        ron::de::from_str::<FakePhaseReply>(reply.as_str()).ok(),
        Some(a_reply())
    );
}

#[test]
fn answer_with_carries_the_attachments_through() {
    let (raw, channel) = Responder::channel();
    CommandResponder::<FakePhase>::new(raw).answer_with(&a_reply(), vec![an_attachment()]);

    let Ok(QaResponse::Outcome(CommandOutcome::Ran { reply, attachments })) = channel.try_recv()
    else {
        unreachable!("answer_with must produce a Ran outcome");
    };
    assert_eq!(
        attachments,
        vec![an_attachment()],
        "the caller's attachment list reaches the outcome unchanged"
    );
    assert_eq!(
        ron::de::from_str::<FakePhaseReply>(reply.as_str()).ok(),
        Some(a_reply())
    );
}

#[test]
fn bad_arguments_produces_the_handler_discovered_argument_fault() {
    let detail = ArgumentFault::new("the fake host cannot write that value".to_owned());
    let (raw, channel) = Responder::channel();
    CommandResponder::<FakePhase>::new(raw).bad_arguments(detail.clone());

    let Ok(QaResponse::Outcome(CommandOutcome::BadArguments {
        detail: sent,
        schema,
    })) = channel.try_recv()
    else {
        unreachable!("bad_arguments must produce a BadArguments outcome");
    };
    assert_eq!(sent, detail, "the handler's own detail reaches the client");
    assert_eq!(
        schema.as_str(),
        shape_text::<<FakePhase as QaCommand>::Args>(),
        "a refusal the handler discovered carries the same argument shape a decode failure \
         carries, so one round trip tells a client what it may send",
    );
}

#[test]
fn unavailable_produces_the_handler_discovered_refusal() {
    let note = RefusalNote::from_static("the fake host discovered this mid-handler");
    let (raw, channel) = Responder::channel();
    CommandResponder::<FakePhase>::new(raw).unavailable(UnavailableCode::WrongState, note.clone());

    assert_eq!(
        channel.try_recv().ok(),
        Some(QaResponse::Outcome(CommandOutcome::Unavailable {
            code: UnavailableCode::WrongState,
            note,
        }))
    );
}

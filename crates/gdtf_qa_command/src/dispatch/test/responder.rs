use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{
        ArtifactPath, AttachmentKind, CommandOutcome, RefusalNote, ReplyAttachment, UnavailableCode,
    },
    message::QaResponse,
};

use crate::{
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

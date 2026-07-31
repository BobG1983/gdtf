//! The typed responder's three answers: a plain reply, a reply carrying files, and the
//! refusal only a handler can discover.

use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{
        ArtifactPath, AttachmentKind, CommandOutcome, RefusalNote, ReplyAttachment, UnavailableCode,
    },
    envelope::QaResponse,
};

use crate::{
    dispatch::CommandResponder,
    test_support::{FakeLevel, FakePhase, FakePhaseReply, FakeReady},
};

/// The reply every case below sends.
fn a_reply() -> FakePhaseReply {
    FakePhaseReply {
        ready: FakeReady::new(true),
        level: FakeLevel::new(3),
    }
}

/// One attachment, so the list the caller passes is distinguishable from an empty one.
fn an_attachment() -> ReplyAttachment {
    ReplyAttachment::new(
        AttachmentKind::Png,
        ArtifactPath::new("fake/shot.png".to_owned()),
    )
}

/// `answer` produces `Ran` with the declared reply and NO attachments.
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
        serde_json::from_str::<FakePhaseReply>(reply.as_str()).ok(),
        Some(a_reply())
    );
}

/// `answer_with` carries the caller's attachment list THROUGH to the outcome.
///
/// Without this, the parameter could be dropped on the floor and every other test in the
/// crate would still pass — no fake command attaches a file.
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
        serde_json::from_str::<FakePhaseReply>(reply.as_str()).ok(),
        Some(a_reply())
    );
}

/// `unavailable` produces the handler-discovered refusal, carrying the code and note it was
/// given.
///
/// Distinct from the route-time refusal `admit()` produces: this one is for a precondition
/// only the handler can see, so it has to be its own observed shape.
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

use cobalt_mcp_command::{
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS, FakeLevel, FakePhase, FakePhaseReply, FakePoint, FakePointPair,
        FakePointReply, FakePointX, FakePointY, FakeReady, fake_app, fake_facts_loaded,
        run_fake_command,
    },
};

use crate::support::{args, plain, ran};

#[test]
fn an_admitted_call_reaches_the_handler_and_answers_the_declared_reply() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &plain(),
    );

    app.update();

    let reply: FakePhaseReply = ran(&channel);
    assert_eq!(
        reply,
        FakePhaseReply {
            ready: FakeReady::new(true),
            level: FakeLevel::new(2),
        }
    );
}

#[test]
fn arguments_decode_into_the_command_s_own_type() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args("(point:(x:3,y:-4))"),
        &plain(),
    );

    app.update();

    let reply: FakePointReply = ran(&channel);
    assert_eq!(
        reply,
        FakePointReply {
            point: FakePointPair::new(FakePointX::new(3), FakePointY::new(-4)),
            level: FakeLevel::new(2),
        }
    );
}

#[test]
fn two_commands_in_one_frame_each_get_their_own_calls() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let phase = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &plain(),
    );
    let point = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args("(point:(x:0,y:0))"),
        &plain(),
    );

    app.update();

    let phase_reply: FakePhaseReply = ran(&phase);
    let point_reply: FakePointReply = ran(&point);
    assert_eq!(phase_reply.level, FakeLevel::new(2));
    assert_eq!(
        point_reply.point,
        FakePointPair::new(FakePointX::new(0), FakePointY::new(0))
    );
}

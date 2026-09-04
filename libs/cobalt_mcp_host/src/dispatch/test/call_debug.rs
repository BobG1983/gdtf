use crate::{
    command::McpCommand,
    dispatch::CommandCall,
    test_support::{
        FakePhase, FakePhaseArgs, FakePoint, FakePointArgs, FakePointPair, FakePointX, FakePointY,
    },
};

#[test]
fn a_queued_call_prints_its_name_and_its_arguments() {
    let call = CommandCall::<FakePoint>::new(FakePointArgs {
        point: FakePointPair::new(FakePointX::new(3), FakePointY::new(-4)),
    });

    let printed = format!("{call:?}");

    assert!(
        printed.contains(FakePoint::NAME.as_str()),
        "the log line must name the command that hung: {printed}"
    );
    assert!(
        printed.contains('3') && printed.contains("-4"),
        "the log line must carry the decoded arguments, not just the name: {printed}"
    );
}

#[test]
fn an_argument_less_call_still_prints_its_name() {
    let printed = format!("{:?}", CommandCall::<FakePhase>::new(FakePhaseArgs {}));

    assert!(
        printed.contains(FakePhase::NAME.as_str()),
        "the log line must name the command that hung: {printed}"
    );
}

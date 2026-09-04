use super::assert_ron_round_trip;
use crate::dev::mcp::wire::sight::{CanEngageNet, CanSeeNet, SightlineNet};

#[test]
fn sightline_flags_round_trip() {
    assert_ron_round_trip(&CanSeeNet::new(true));
    assert_ron_round_trip(&CanSeeNet::new(false));
    assert_ron_round_trip(&CanEngageNet::new(true));
    assert_ron_round_trip(&CanEngageNet::new(false));
}

#[test]
fn every_sightline_answer_round_trips() {
    for answer in [
        SightlineNet::NoShooter,
        SightlineNet::Answered {
            can_see:    CanSeeNet::new(true),
            can_engage: CanEngageNet::new(false),
        },
    ] {
        match answer {
            SightlineNet::NoShooter | SightlineNet::Answered { .. } => {}
        }
        assert_ron_round_trip(&answer);
    }
}

#[test]
fn seeing_and_engaging_stay_separate_fields() {
    let Ok(text) = ron::ser::to_string(&SightlineNet::Answered {
        can_see:    CanSeeNet::new(true),
        can_engage: CanEngageNet::new(false),
    }) else {
        unreachable!("a sightline answer serializes to compact RON");
    };
    assert_eq!(
        text, "Answered(can_see:true,can_engage:false)",
        "collapsing the two answers into one would lose the one a caller needs",
    );
}

//! Exhaustive witnesses for the ganger card's enums — life state, injury severity, and
//! body part (GTW-734).

use crate::{
    test_support::assert_ron_round_trip,
    view::{BodyPartNet, LifeStateNet, SeverityNet},
};

/// Every [`LifeStateNet`] variant round-trips; the witness forces new variants in.
#[test]
fn life_state_round_trips_every_variant() {
    for life in [
        LifeStateNet::Alive,
        LifeStateNet::Downed,
        LifeStateNet::Dead,
    ] {
        match life {
            LifeStateNet::Alive | LifeStateNet::Downed | LifeStateNet::Dead => {}
        }
        assert_ron_round_trip(&life);
    }
}

/// Every [`SeverityNet`] bucket round-trips; the witness forces new variants in.
#[test]
fn severity_round_trips_every_variant() {
    for severity in [
        SeverityNet::None,
        SeverityNet::Minor,
        SeverityNet::Major,
        SeverityNet::Critical,
        SeverityNet::Fatal,
    ] {
        match severity {
            SeverityNet::None
            | SeverityNet::Minor
            | SeverityNet::Major
            | SeverityNet::Critical
            | SeverityNet::Fatal => {}
        }
        assert_ron_round_trip(&severity);
    }
}

/// Every [`BodyPartNet`] round-trips; the witness forces new variants in.
#[test]
fn body_part_round_trips_every_variant() {
    for part in [
        BodyPartNet::Head,
        BodyPartNet::Torso,
        BodyPartNet::LeftArm,
        BodyPartNet::RightArm,
        BodyPartNet::LeftLeg,
        BodyPartNet::RightLeg,
    ] {
        match part {
            BodyPartNet::Head
            | BodyPartNet::Torso
            | BodyPartNet::LeftArm
            | BodyPartNet::RightArm
            | BodyPartNet::LeftLeg
            | BodyPartNet::RightLeg => {}
        }
        assert_ron_round_trip(&part);
    }
}

use gdtf_battle_sim::{armor::BodyPart, inflicted_wound::InflictedWound, severity::Severity};

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::wound::{
    BodyPartNet, InjuryNameNet, InjuryNet, SeverityNet, WoundNet,
};

#[test]
fn every_severity_round_trips() {
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

#[test]
fn every_body_part_round_trips() {
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

#[test]
fn wounds_and_injuries_round_trip() {
    assert_ron_round_trip(&WoundNet::new(SeverityNet::Major, BodyPartNet::Torso));
    assert_ron_round_trip(&InjuryNameNet::new("shattered_knee".to_owned()));
    assert_ron_round_trip(&InjuryNet::new(
        InjuryNameNet::new("shattered_knee".to_owned()),
        BodyPartNet::LeftLeg,
        SeverityNet::Critical,
    ));
}

#[test]
fn a_sim_wound_mirrors_onto_the_wire() {
    let mirrored = WoundNet::from_sim(InflictedWound::new(Severity::Fatal, BodyPart::Head));
    assert_eq!(
        mirrored,
        WoundNet::new(SeverityNet::Fatal, BodyPartNet::Head),
        "the mirror must carry the sim's own severity and location, not a default",
    );
}

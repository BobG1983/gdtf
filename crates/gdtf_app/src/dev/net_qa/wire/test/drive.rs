use gdtf_battle_sim::procgen::ProcgenStage;

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    key::FocusStepNet,
    misc::{AutoRunNet, ProcgenStageNet, StepperCommandNet},
};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a drive command serializes to compact RON");
    };
    text
}

#[test]
fn focus_steps_round_trip_every_direction() {
    for step in [
        FocusStepNet::Next,
        FocusStepNet::Prev,
        FocusStepNet::Left,
        FocusStepNet::Right,
    ] {
        match step {
            FocusStepNet::Next | FocusStepNet::Prev | FocusStepNet::Left | FocusStepNet::Right => {}
        }
        assert_ron_round_trip(&step);
    }
}

#[test]
fn stepper_commands_round_trip_every_variant() {
    for command in [
        StepperCommandNet::Next,
        StepperCommandNet::Auto {
            running: AutoRunNet::new(true),
        },
        StepperCommandNet::Auto {
            running: AutoRunNet::new(false),
        },
        StepperCommandNet::Skip,
    ] {
        match command {
            StepperCommandNet::Next | StepperCommandNet::Auto { .. } | StepperCommandNet::Skip => {}
        }
        assert_ron_round_trip(&command);
    }
    assert_ron_round_trip(&AutoRunNet::new(true));
    assert_ron_round_trip(&AutoRunNet::new(false));
}

#[test]
fn procgen_stages_round_trip_every_variant() {
    for stage in [
        ProcgenStage::Assemble,
        ProcgenStage::Fill,
        ProcgenStage::Emit,
        ProcgenStage::Done,
    ] {
        assert_ron_round_trip(&ProcgenStageNet::from_stage(stage));
    }
    assert_eq!(
        ProcgenStageNet::from_stage(ProcgenStage::Fill),
        ProcgenStageNet::Fill,
        "the wire mirror must name the same stage the sim is on",
    );
}

/// a round trip alone cannot see a lost `#[serde(transparent)]` or a `serde(rename)`.
#[test]
fn drive_values_serialize_under_their_own_names() {
    assert_eq!(encoded(&FocusStepNet::Next), "Next");
    assert_eq!(encoded(&FocusStepNet::Right), "Right");
    assert_eq!(encoded(&AutoRunNet::new(true)), "true");
    assert_eq!(encoded(&AutoRunNet::new(false)), "false");
    assert_eq!(encoded(&StepperCommandNet::Skip), "Skip");
    assert_eq!(
        encoded(&StepperCommandNet::Auto {
            running: AutoRunNet::new(true),
        }),
        "Auto(running:true)",
    );
}

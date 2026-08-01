//! Round-trip + wire-text pins for the two drive vocabularies a caller chooses —
//! [`FocusStepNet`] and [`StepperCommandNet`] with its [`AutoRunNet`] flag (GTW-944).
//!
//! Both were deleted from the shared protocol crate by GTW-943 and re-minted in `wire/`
//! here, so their old round-trip cases went with them. These are the replacements.

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    key::FocusStepNet,
    misc::{AutoRunNet, StepperCommandNet},
};

/// The compact RON `value` encodes to.
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom) if it cannot encode.
fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a drive command serializes to compact RON");
    };
    text
}

/// Every [`FocusStepNet`] direction round-trips; the wildcard-free witness fails to compile
/// the moment a fifth is added, forcing it into this table first.
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

/// Every [`StepperCommandNet`] variant round-trips, including BOTH [`AutoRunNet`] states —
/// the Auto arm carries a payload, so one of the two proves nothing about the other.
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

/// Each drive value rides the wire under its own name, and [`AutoRunNet`] as a BARE `bool` —
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

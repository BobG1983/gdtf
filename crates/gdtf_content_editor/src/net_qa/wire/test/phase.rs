use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{EditorState, net_qa::wire::EditorPhaseNet};

const PHASES: [EditorState; 2] = [EditorState::Load, EditorState::Editing];

#[test]
fn every_editor_state_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(PHASES.len());
    for state in &PHASES {
        let mirrored = format!("{:?}", EditorPhaseNet::from_state(state));
        assert_eq!(
            mirrored,
            format!("{state:?}"),
            "the wire mirror of {state:?} must carry that state's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{state:?} maps onto {mirrored}, which another state already claims — two states \
             that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_editor_phase_arm_round_trips() {
    for state in &PHASES {
        assert_ron_round_trip(&EditorPhaseNet::from_state(state));
    }
}

#[test]
fn the_editor_phase_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorPhaseNet>("EditorPhaseNet");
}

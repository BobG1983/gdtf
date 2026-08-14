use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{EditorMode, net_qa::wire::EditorModeNet};

#[test]
fn every_editor_mode_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(EditorMode::TAB_ORDER.len());
    for mode in EditorMode::TAB_ORDER {
        let mirrored = format!("{:?}", EditorModeNet::from_mode(mode));
        assert_eq!(
            mirrored,
            format!("{mode:?}"),
            "the wire mirror of {mode:?} must carry that mode's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{mode:?} maps onto {mirrored}, which another mode already claims — two modes that \
             read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_editor_mode_arm_round_trips() {
    for mode in EditorMode::TAB_ORDER {
        assert_ron_round_trip(&EditorModeNet::from_mode(mode));
    }
}

#[test]
fn the_editor_mode_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorModeNet>("EditorModeNet");
}

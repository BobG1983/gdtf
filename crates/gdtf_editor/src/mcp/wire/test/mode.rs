use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{EditorMode, mcp::wire::EditorModeNet};

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
fn every_family_arm_round_trips() {
    for family in EditorModeNet::CONTENT_FAMILIES {
        assert_ron_round_trip(&family);
    }
}

#[test]
fn every_family_arm_is_listed_once_and_the_prefab_tab_is_not_one() {
    let mut seen: Vec<String> = Vec::with_capacity(EditorModeNet::CONTENT_FAMILIES.len());
    for family in EditorModeNet::CONTENT_FAMILIES {
        let named = format!("{family:?}");
        assert!(
            !seen.contains(&named),
            "{named} appears twice in EditorModeNet::CONTENT_FAMILIES, so a full read would \
             answer that family twice and leave another one out",
        );
        assert_ne!(
            family,
            EditorModeNet::Prefab,
            "the Prefab tab is the map canvas and owns no registry, so a families read that \
             walked it would answer an empty row instead of refusing",
        );
        seen.push(named);
    }
    assert_eq!(
        EditorModeNet::CONTENT_FAMILIES.len() + 1,
        EditorMode::TAB_ORDER.len(),
        "every tab but Prefab owns a content family, so a new tab nobody classified is missing \
         from CONTENT_FAMILIES",
    );
}

#[test]
fn the_editor_mode_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorModeNet>("EditorModeNet");
}

#[test]
fn every_editor_mode_reads_back_as_the_mode_it_mirrored() {
    for mode in EditorMode::TAB_ORDER {
        assert_eq!(
            EditorModeNet::from_mode(mode).to_mode(),
            mode,
            "a client's mode must come back as the editor's own, or a write command would act on \
             a different tab than the one it was asked for",
        );
    }
}

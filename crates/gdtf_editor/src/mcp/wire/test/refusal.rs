use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::mcp::wire::EditorRefusalNet;

const REFUSALS: [EditorRefusalNet; 4] = [
    EditorRefusalNet::NoNewAction,
    EditorRefusalNet::ThemeNewIsUndoneBySync,
    EditorRefusalNet::NameBelongsToPrefabOnly,
    EditorRefusalNet::PrefabNeedsAName,
];

#[test]
fn every_refusal_arm_round_trips() {
    for refusal in REFUSALS {
        assert_ron_round_trip(&refusal);
    }
}

#[test]
fn every_refusal_reads_as_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(REFUSALS.len());
    for refusal in REFUSALS {
        let named = format!("{refusal:?}");
        assert!(
            !seen.contains(&named),
            "{named} is claimed by another refusal — two reasons that read the same on the wire \
             are indistinguishable to a client",
        );
        seen.push(named);
    }
}

#[test]
fn the_list_holds_every_arm_the_enum_declares() {
    for refusal in REFUSALS {
        let named = match refusal {
            EditorRefusalNet::NoNewAction => "NoNewAction",
            EditorRefusalNet::ThemeNewIsUndoneBySync => "ThemeNewIsUndoneBySync",
            EditorRefusalNet::NameBelongsToPrefabOnly => "NameBelongsToPrefabOnly",
            EditorRefusalNet::PrefabNeedsAName => "PrefabNeedsAName",
        };
        assert_eq!(
            format!("{refusal:?}"),
            named,
            "the match above names every arm and carries no wildcard, so an arm this list does \
             not hold stops the crate compiling rather than going unread on the wire",
        );
    }
}

#[test]
fn the_refusal_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorRefusalNet>("EditorRefusalNet");
}

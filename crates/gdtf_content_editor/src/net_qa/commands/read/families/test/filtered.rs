use crate::net_qa::{commands::read::families::filter::families_to_answer, wire::EditorModeNet};

#[test]
fn a_read_without_a_filter_answers_every_content_family_in_order() {
    assert_eq!(
        families_to_answer(None),
        Ok(EditorModeNet::CONTENT_FAMILIES.to_vec()),
        "an unfiltered read answers every family the wire enum names, once each and in its own \
         order, so a client can walk the reply without checking for gaps",
    );
}

#[test]
fn a_filter_answers_the_named_family_alone() {
    assert_eq!(
        families_to_answer(Some(EditorModeNet::Armor)),
        Ok(vec![EditorModeNet::Armor]),
        "a filtered read answers the named family and nothing else, so a client asking for one \
         family does not have to sift the others out",
    );
}

#[test]
fn the_prefab_tab_is_refused_because_it_carries_no_family() {
    let refused = families_to_answer(Some(EditorModeNet::Prefab));
    let detail = refused.as_ref().err().map_or("", |fault| fault.as_str());
    assert!(
        detail.contains("Prefab"),
        "the Prefab tab owns no registry, so it is refused with a detail naming that tab rather \
         than answered as an empty row, got {refused:?}",
    );
}

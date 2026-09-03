//! A delete is offered only on the screen its entry names.

use crate::{
    delete::{
        entries::{PREFAB_FAMILY, WEIGHTING_FAMILY, prefab_delete_entry, weighting_delete_entry},
        offered::entries_offered_on,
        registry::DeleteRegistry,
    },
    mode::{EditorMode, InjurySubTab},
};

// The registry the editor builds: the prefab entry and the weighting entry.
fn registry() -> DeleteRegistry {
    let mut registry = DeleteRegistry::default();
    registry.add(prefab_delete_entry());
    registry.add(weighting_delete_entry());
    registry
}

// The family labels offered on one screen, in registration order.
fn labels_on(mode: EditorMode, sub_tab: Option<InjurySubTab>) -> Vec<String> {
    entries_offered_on(&registry(), mode, sub_tab)
        .into_iter()
        .map(|entry| (**entry.family()).clone())
        .collect()
}

#[test]
fn the_weighting_delete_is_offered_on_the_injury_tables_sub_tab() {
    assert_eq!(
        labels_on(EditorMode::Injury, Some(InjurySubTab::Tables)),
        vec![WEIGHTING_FAMILY.to_owned()],
        "the weighting table is the Tables sub-tab's own record, so its delete is the one \
         offered there",
    );
}

#[test]
fn no_delete_is_offered_on_the_injury_def_sub_tab() {
    assert_eq!(
        labels_on(EditorMode::Injury, Some(InjurySubTab::Def)),
        Vec::<String>::new(),
        "the injury def has no delete in this build, so the Def sub-tab offers none",
    );
}

#[test]
fn the_prefab_delete_is_offered_on_the_prefab_tab() {
    assert_eq!(
        labels_on(EditorMode::Prefab, None),
        vec![PREFAB_FAMILY.to_owned()],
        "the prefab canvas is where a prefab is deleted from",
    );
}

use crate::mode::{EditorMode, InjurySubTab};

#[test]
fn injury_owns_exactly_one_top_level_tab() {
    let injury_tabs = EditorMode::TAB_ORDER
        .iter()
        .filter(|mode| mode.tab_label() == "INJURY")
        .count();
    assert_eq!(
        injury_tabs, 1,
        "the Injury workflow is one top-level tab; splitting it into two would give the tab bar \
         a second INJURY entry, got {injury_tabs}",
    );
}

#[test]
fn only_the_injury_tab_itself_carries_a_sub_tab_label() {
    let promoted: Vec<(&'static str, &'static str)> = InjurySubTab::TAB_ORDER
        .iter()
        .flat_map(|sub_tab| {
            let label = sub_tab.tab_label().to_uppercase();
            EditorMode::TAB_ORDER
                .iter()
                .filter(move |mode| mode.tab_label() == label)
                .map(|mode| (sub_tab.tab_label(), mode.tab_label()))
        })
        .collect();
    assert_eq!(
        promoted,
        vec![("Injury", "INJURY")],
        "the Injury sub-tabs live inside the Injury tab. The Def sub-tab shares that tab's own \
         label, and no other sub-tab may reach the top-level tab bar",
    );
}

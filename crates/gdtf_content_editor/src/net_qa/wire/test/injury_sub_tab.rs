use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{InjurySubTab, net_qa::wire::InjurySubTabNet};

#[test]
fn every_injury_sub_tab_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(InjurySubTab::TAB_ORDER.len());
    for sub_tab in InjurySubTab::TAB_ORDER {
        let mirrored = format!("{:?}", InjurySubTabNet::from_sub_tab(sub_tab));
        assert_eq!(
            mirrored,
            format!("{sub_tab:?}"),
            "the wire mirror of {sub_tab:?} must carry that sub-tab's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{sub_tab:?} maps onto {mirrored}, which another sub-tab already claims. Two \
             sub-tabs that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_injury_sub_tab_arm_round_trips() {
    for sub_tab in InjurySubTab::TAB_ORDER {
        assert_ron_round_trip(&InjurySubTabNet::from_sub_tab(sub_tab));
    }
}

#[test]
fn the_injury_sub_tab_traces_a_usable_shape() {
    assert_schema_is_usable::<InjurySubTabNet>("InjurySubTabNet");
}

#[test]
fn every_injury_sub_tab_reads_back_as_the_sub_tab_it_mirrored() {
    for sub_tab in InjurySubTab::TAB_ORDER {
        assert_eq!(
            InjurySubTabNet::from_sub_tab(sub_tab).to_sub_tab(),
            sub_tab,
            "a client's sub-tab must come back as the editor's own, or the write would open a \
             different sub-tab than the one it was asked for",
        );
    }
}

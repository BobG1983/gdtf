//! Round-trip pins for the menu enumeration handout folded into the app-flow snapshot
//! (GTW-787).

use crate::{
    ids::FocusTargetNet,
    test_support::assert_ron_round_trip,
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, MenuIdNet, MenuItemEnabledNet,
        MenuItemLabelNet, MenuItemView, MenuView, RequestKindNet,
    },
};

/// The app-flow snapshot's folded menu view round-trips — the identity plus a mixed
/// enabled/disabled item set (GTW-787).
#[test]
fn app_flow_menu_view_round_trips() {
    let menu = MenuView::new(
        MenuIdNet::new("MainMenu".to_owned()),
        vec![
            MenuItemView::new(
                FocusTargetNet::new(11),
                MenuItemLabelNet::new("Battlescape".to_owned()),
                MenuItemEnabledNet::new(true),
            ),
            MenuItemView::new(
                FocusTargetNet::new(12),
                MenuItemLabelNet::new("HiveScape".to_owned()),
                MenuItemEnabledNet::new(false),
            ),
        ],
    );
    assert_ron_round_trip(&menu);
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(false),
        RequestKindNet::ALL.to_vec(),
        CaughtUpNet::new(true),
        Some(menu),
        None,
    ));
}

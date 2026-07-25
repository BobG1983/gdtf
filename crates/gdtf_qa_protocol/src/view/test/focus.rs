//! Round-trip pins for the focus-navigable control enumeration handout folded into the
//! app-flow snapshot (GTW-802).

use crate::{
    ids::FocusTargetNet,
    test_support::assert_ron_round_trip,
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, FocusView, FocusableCheckedNet,
        FocusableEnabledNet, FocusableKindNet, FocusableLabelNet, FocusableView, FocusedNet,
        RequestKindNet,
    },
};

/// A representative focus snapshot — the Options screen's shape: a checked checkbox that
/// currently holds focus, and a plain button that does not.
fn a_focus_view() -> FocusView {
    FocusView::new(
        Some(FocusTargetNet::new(21)),
        vec![
            FocusableView::new(
                FocusTargetNet::new(21),
                FocusableLabelNet::new("Sound".to_owned()),
                FocusableKindNet::Checkbox,
                FocusableEnabledNet::new(true),
                Some(FocusableCheckedNet::new(true)),
                FocusedNet::new(true),
            ),
            FocusableView::new(
                FocusTargetNet::new(22),
                FocusableLabelNet::new("Continue".to_owned()),
                FocusableKindNet::Button,
                FocusableEnabledNet::new(true),
                None,
                FocusedNet::new(false),
            ),
        ],
    )
}

/// The focus snapshot — and thus every focusable row, including a checkbox's `checked`
/// value and the `None` a non-checkbox carries — round-trips identically, so the token a
/// client echoes back to `FocusControl` survives the wire.
#[test]
fn focus_view_round_trips() {
    assert_ron_round_trip(&a_focus_view());
    for focusable in a_focus_view().focusables {
        assert_ron_round_trip(&focusable);
    }
}

/// The app-flow snapshot's folded focus view round-trips, alongside the `None` form a
/// screen with no focus graph reports.
#[test]
fn app_flow_focus_view_round_trips() {
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(false),
        RequestKindNet::ALL.to_vec(),
        CaughtUpNet::new(true),
        None,
        Some(a_focus_view()),
    ));
}

/// Every [`FocusableKindNet`] round-trips; the witness forces new variants in, so a future
/// control kind cannot silently read as a button.
#[test]
fn focusable_kind_round_trips_every_variant() {
    for kind in [
        FocusableKindNet::Button,
        FocusableKindNet::Checkbox,
        FocusableKindNet::Other,
    ] {
        match kind {
            FocusableKindNet::Button | FocusableKindNet::Checkbox | FocusableKindNet::Other => {}
        }
        assert_ron_round_trip(&kind);
    }
}

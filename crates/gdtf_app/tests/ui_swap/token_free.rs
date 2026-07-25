//! The token-free projection of a battle snapshot — what the cross-stack parity comparison
//! actually compares.
//!
//! Every wire token in a `BattleView` is `Entity::to_bits`, so it depends on how many
//! entities the app happened to spawn before it. The two parity runs differ there by
//! construction: the `bevy_ui` run has the swap panel's entities on screen and the egui run
//! does not, which shifts every later allocation. Comparing raw tokens across the two runs
//! would therefore assert an allocation coincidence rather than parity — so [`token_free`]
//! scrubs each token to one constant and sorts the token-keyed lists, and [`selected_name`]
//! says WHICH ganger is selected by name instead of by token.

use gdtf_qa_protocol::{
    ids::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
    view::{BattleView, SelectionView},
};

/// The constant every wire token is scrubbed to before the two runs are compared.
const NO_TOKEN: u64 = 0;

/// The selected ganger's NAME — the token-free way to say WHICH ganger is selected.
pub(crate) fn selected_name(view: &BattleView) -> Option<String> {
    let token = view.selection.selected?;
    view.gangers
        .iter()
        .find(|card| card.token == token)
        .map(|card| (*card.name).clone())
}

/// The snapshot with every wire token scrubbed and every token-keyed list sorted, plus the
/// harness's own stack report dropped (the ONE field the two runs must differ in).
pub(crate) fn token_free(view: &BattleView) -> BattleView {
    let mut view = view.clone();
    view.ui_stack = None;
    for card in &mut view.gangers {
        card.token = GangerToken::new(NO_TOKEN);
    }
    view.gangers.sort_by_key(|card| format!("{card:?}"));
    for door in &mut view.terrain.doors {
        door.token = DoorToken::new(NO_TOKEN);
    }
    view.terrain.doors.sort_by_key(|door| format!("{door:?}"));
    for spot in &mut view.terrain.emplacements {
        spot.token = EmplacementToken::new(NO_TOKEN);
    }
    view.terrain
        .emplacements
        .sort_by_key(|spot| format!("{spot:?}"));
    for button in &mut view.buttons {
        button.token = FocusTargetNet::new(NO_TOKEN);
    }
    view.buttons.sort_by_key(|button| *button.order);
    view.selection =
        SelectionView::new(view.selection.selected.map(|_| GangerToken::new(NO_TOKEN)));
    view
}

//! The focus-navigable HUD button token handout (GTW-789, the OBSERVE side of GTW-782's
//! focus nav).
//!
//! [`panel_button_views`] walks the [`SnapshotWorld`]'s [`PanelNavOrder`] button query,
//! mints each button's entity into a [`FocusTargetNet`] (the token
//! [`SetFocus`](gdtf_qa_protocol::intent::NetIntent::SetFocus) echoes back), and pairs it
//! with the button's Tab-chain ordinal and its human-readable label. Without this handout
//! `SetFocus` had no panel-button token to name — the only tokens on the wire were
//! gangers / doors / emplacements — so GTW-782's keyboard focus ring was un-drivable over
//! the `net_qa` wire.
//!
//! The label is read from what the button already carries:
//! [`spawn_button`](gdtf_ui::spawn_button) puts the caption on a `Text` CHILD (not the
//! button root), so [`button_label`] keys the button's child entities through the
//! [`button_texts`](SnapshotWorld::button_texts) query — no new state is authored.

use bevy::prelude::*;
use gdtf_battle_input::PanelNavOrder;
use gdtf_qa_protocol::{
    ids::FocusTargetNet,
    view::{PanelButtonLabelNet, PanelButtonView, PanelNavOrderNet},
};

use super::read::SnapshotWorld;

/// Project every focus-navigable battlescape HUD button into its [`PanelButtonView`] — its
/// focus token, Tab-chain ordinal, and label.
///
/// Sorted by `(PanelNavOrder, Entity)` — the exact deterministic order the GTW-782 focus-nav
/// topology walks its Tab chain (`rebuild_panel_nav_topology`), so the wire list reads
/// left-to-right in traversal order. During a live battle the only [`PanelNavOrder`]-bearing
/// entities are the HUD buttons (the menu's are despawned), so the query is battle-scoped by
/// the builder's live-battle gate.
pub(super) fn panel_button_views(world: &SnapshotWorld) -> Vec<PanelButtonView> {
    let mut rows: Vec<(PanelNavOrder, Entity, PanelButtonLabelNet)> = world
        .buttons
        .iter()
        .map(|(entity, order, children)| (*order, entity, button_label(world, children)))
        .collect();
    rows.sort_by_key(|(order, entity, _)| (*order, *entity));
    rows.into_iter()
        .map(|(order, entity, label)| {
            PanelButtonView::new(
                FocusTargetNet::new(entity.to_bits()),
                PanelNavOrderNet::new(*order),
                label,
            )
        })
        .collect()
}

/// Read a button's on-screen caption into a [`PanelButtonLabelNet`] — the first child
/// entity that carries a [`Text`] node ([`spawn_button`](gdtf_ui::spawn_button) spawns
/// exactly one caption child). A button with no readable caption child yields an empty
/// label rather than being dropped (fail-open — the token + ordinal are still useful).
fn button_label(world: &SnapshotWorld, children: Option<&Children>) -> PanelButtonLabelNet {
    let caption = children
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| world.button_texts.get(child).ok())
        })
        .map(|text| text.0.clone())
        .unwrap_or_default();
    PanelButtonLabelNet::new(caption)
}

//! The **Stabilize** contextual act's panel-layer module (GTW-294 / GTW-571): marker,
//! descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::StabilizeAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Stabilize** contextual button (GTW-294) — the act that arrests a downed
    /// neighbour's bleed-out.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// [`offer_stabilize`] names a target. A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StabilizeButton;
}

impl ContextualPanelAct for StabilizeAct {
    type Marker = StabilizeButton;

    const SLOT: PanelSlot = PanelSlot::new(1);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Stabilize")
    }
}

/// The candidate-neighbour reads the Stabilize scan needs — each ganger's identity,
/// cell, life, gang, and the optional [`BleedingOut`] condition. A named alias for clippy
/// `type_complexity` legibility.
type StabilizeCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static BleedingOut>,
);

/// OFFERS the Stabilize act: the first [`LifeState::Downed`] ALLY (same faction) that
/// is currently [`BleedingOut`] (its bleed clock still runs) 8-adjacent to the
/// selected actor, or nothing (GTW-294).
///
/// The actor resolves from the [`SelectedShooter`]'s `(Position, Faction)`; any miss
/// clears the offer — fail-closed. The sim's `stabilize_downed` faction gate is the
/// authoritative re-check when the act fires; this only decides what to OFFER. Writes
/// [`ContextualOffer`] via `set_if_neq` (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_stabilize(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<StabilizeCandidates>,
    mut offer: ResMut<ContextualOffer<StabilizeAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
            candidates
                .iter()
                .find(|(_, pos, life, faction, bleeding_out)| {
                    **life == LifeState::Downed
                        && **faction == *actor_faction
                        && bleeding_out.is_some()
                        && *is_8_adjacent(*actor_pos, **pos)
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

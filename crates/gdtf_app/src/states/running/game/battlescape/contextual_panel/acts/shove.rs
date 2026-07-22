//! The **Shove** contextual act's panel-layer module (GTW-525 / GTW-571): marker,
//! descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ShoveAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Shove** contextual button (GTW-525) — the deliberate knock-back act on an
    /// 8-adjacent, ALIVE, opposing ganger.
    ///
    /// A UNIVERSAL act available to EVERY ganger (NO weapon requirement — a pure-displacement
    /// shove, not a weapon strike). Spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic button
    /// spawn and revealed IN PLACE by the act's visibility toggle when `offer_shove` names
    /// a target — a WEAKER gate than Melee's (no LOS required: a shove is contact, not a
    /// sighted strike). A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ShoveButton;
}

impl ContextualPanelAct for ShoveAct {
    type Marker = ShoveButton;

    const SLOT: PanelSlot = PanelSlot::new(3);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Shove")
    }
}

/// The candidate-neighbour reads the Shove scan needs — each ganger's identity, cell,
/// life, and gang. A named alias for clippy `type_complexity` legibility.
type ShoveCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
);

/// OFFERS the Shove act: the first 8-adjacent, [`LifeState::Alive`] (a fresh
/// `is_active` — NOT Downed / Dead), OPPOSING ganger, or nothing (GTW-525).
///
/// A WEAKER gate than Melee's: a shove is CONTACT, so it needs NO LOS check and NO
/// weapon — ANY ganger can shove any alive opposing neighbour (which is why the actor
/// resolve needs only `(Position, Faction)`, no stance / facing). The sim's
/// `dispatch_shove` gate is the authoritative re-check when the act fires; this only
/// decides what to OFFER. Writes [`ContextualOffer`] via `set_if_neq`
/// (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_shove(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<ShoveCandidates>,
    mut offer: ResMut<ContextualOffer<ShoveAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
            candidates
                .iter()
                .find(|(_, pos, life, faction)| {
                    **life == LifeState::Alive
                        && **faction != *actor_faction
                        && *is_8_adjacent(*actor_pos, **pos)
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

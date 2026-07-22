//! The **Execute** contextual act's panel-layer module (GTW-294 / GTW-571): marker,
//! descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExecuteAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Execute** contextual button (GTW-294) — the coup-de-grâce act on a downed
    /// neighbour.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// `offer_execute` names a target. A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExecuteButton;
}

impl ContextualPanelAct for ExecuteAct {
    type Marker = ExecuteButton;

    const SLOT: PanelSlot = PanelSlot::new(0);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Execute")
    }
}

/// The candidate-neighbour reads the Execute scan needs — each ganger's identity, cell,
/// life, and gang. A named alias for clippy `type_complexity` legibility.
type ExecuteCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
);

/// OFFERS the Execute act: the first [`LifeState::Downed`] ENEMY (different faction)
/// 8-adjacent to the selected actor, or nothing (GTW-294).
///
/// The actor resolves from the [`SelectedShooter`]'s `(Position, Faction)`; any miss
/// (no selection, or a selection lacking the reads) clears the offer — fail-closed, no
/// panic. The sim's `execute_downed` faction gate is the authoritative re-check when
/// the act fires; this only decides what to OFFER. Writes [`ContextualOffer`] via
/// `set_if_neq` (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_execute(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<ExecuteCandidates>,
    mut offer: ResMut<ContextualOffer<ExecuteAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
            candidates
                .iter()
                .find(|(_, pos, life, faction)| {
                    **life == LifeState::Downed
                        && **faction != *actor_faction
                        && *is_8_adjacent(*actor_pos, **pos)
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

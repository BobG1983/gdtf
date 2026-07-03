//! The **Enter Emplacement** contextual act's panel-layer module (GTW-543 / GTW-571):
//! marker, descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::EnterEmplacementAct};
use gdtf_battle_sim::{
    EmplacementState,
    downed_acts::is_8_adjacent,
    entity::TerrainCell,
    ganger::{Faction, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Enter Emplacement** contextual button (GTW-543) — the act that mans an
    /// 8-adjacent VACANT weapon emplacement.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// [`offer_enter_emplacement`] names a VACANT emplacement the selected PLAYER actor is
    /// 8-adjacent to (F4 player-only). A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EnterEmplacementButton;
}

impl ContextualPanelAct for EnterEmplacementAct {
    type Marker = EnterEmplacementButton;

    const SLOT: PanelSlot = PanelSlot::new(5);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Enter")
    }
}

/// The candidate-EMPLACEMENT reads the enter scan needs — each weapon-emplacement
/// terrain entity's handle, its [`EmplacementState`] (only a VACANT one is offered),
/// and its [`TerrainCell`] (the cell the actor must be 8-adjacent to). An EMPLACEMENT
/// is any terrain entity carrying an [`EmplacementState`] — the GTW-543 mechanism
/// attaches it only to emplacement pieces, so the query never matches a ganger or a
/// door. A named alias for clippy `type_complexity`.
type EnterEmplacementReads = (Entity, &'static EmplacementState, &'static TerrainCell);

/// OFFERS the Enter Emplacement act: the first 8-adjacent
/// [`EmplacementState::Vacant`](gdtf_battle_sim::EmplacementState) weapon emplacement,
/// or nothing (GTW-543).
///
/// An occupied emplacement is NOT offered (it already has an operator); F4 is
/// PLAYER-ONLY (the selection is a player-faction ganger — `With<Faction>` on the actor
/// resolve, the same presence the old shared resolve required). The reach check builds
/// a [`Position`] AT the emplacement's cell (the actor-vs-cell idiom the sim uses). The
/// sim's `dispatch_enter_emplacement` re-gate + TU spend are authoritative; this only
/// decides what to OFFER. Writes [`ContextualOffer`] via `set_if_neq`
/// (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_enter_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<&Position, With<Faction>>,
    emplacements: Query<EnterEmplacementReads>,
    mut offer: ResMut<ContextualOffer<EnterEmplacementAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|actor_pos| {
            emplacements
                .iter()
                .find(|(_, state, emplacement_cell)| {
                    !state.is_occupied()
                        && is_8_adjacent(*actor_pos, Position::new(***emplacement_cell))
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

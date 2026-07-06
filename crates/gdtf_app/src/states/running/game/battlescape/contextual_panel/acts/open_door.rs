//! The **Open Door** contextual act's panel-layer module (GTW-315 / GTW-571): marker,
//! descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::OpenDoorAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    entity::TerrainCell,
    ganger::{Faction, Position},
    openable::OpenState,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Open Door** contextual button (GTW-294 scaffold; GTW-315 live) — the act
    /// that opens an 8-adjacent CLOSED door.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// [`offer_open_door`] names a target — the first 8-adjacent openable terrain entity in
    /// the [`OpenState::Closed`](gdtf_battle_sim::openable::OpenState) state (the button always OPENS;
    /// closing is not offered, and F4 is PLAYER-ONLY). A unit marker: presence on an entity
    /// is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OpenDoorButton;
}

impl ContextualPanelAct for OpenDoorAct {
    type Marker = OpenDoorButton;

    const SLOT: PanelSlot = PanelSlot::new(4);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Open Door")
    }
}

/// The candidate-DOOR reads the open-door scan needs — each openable terrain entity's
/// handle, its [`OpenState`] (only a CLOSED door is offered), and its [`TerrainCell`]
/// (the cell the actor must be 8-adjacent to). A DOOR is any terrain entity carrying an
/// [`OpenState`] — the GTW-503 openable mechanism attaches it only to openable pieces,
/// so the query never matches a ganger. A named alias for clippy `type_complexity`.
type DoorReads = (Entity, &'static OpenState, &'static TerrainCell);

/// OFFERS the Open Door act: the first 8-adjacent openable terrain entity in the
/// [`OpenState::Closed`] state, or nothing (GTW-315).
///
/// The button always OPENS — an already-open door is NOT offered (closing is not a
/// contextual act), and F4 is PLAYER-ONLY (the selection is a player-faction ganger,
/// gated by `With<Faction>` on the actor resolve — the same presence the old shared
/// resolve required). The reach check builds a [`Position`] AT the door's cell (the
/// actor-vs-cell idiom the sim's `dispatch_open_door` uses). The sim's re-gate + TU
/// spend are authoritative; this only decides what to OFFER. Writes
/// [`ContextualOffer`] via `set_if_neq` (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_open_door(
    selected: Res<SelectedShooter>,
    actors: Query<&Position, With<Faction>>,
    doors: Query<DoorReads>,
    mut offer: ResMut<ContextualOffer<OpenDoorAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|actor_pos| {
            doors
                .iter()
                .find(|(_, open_state, door_cell)| {
                    !open_state.is_open() && is_8_adjacent(*actor_pos, Position::new(***door_cell))
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

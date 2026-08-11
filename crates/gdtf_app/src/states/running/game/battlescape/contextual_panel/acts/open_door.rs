use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::OpenDoorAct};
use gdtf_battle_sim::{
    acts::{downed::is_8_adjacent, open_door_tu_cost},
    entity::TerrainCell,
    ganger::{Faction, Position, Tu},
    openable::OpenState,
    tu::can_spend_tu,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that opens or closes an adjacent door.
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

type DoorReads = (Entity, &'static OpenState, &'static TerrainCell);

type OpenDoorActorReads = (&'static Position, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_open_door(
    selected: Res<SelectedShooter>,
    actors: Query<OpenDoorActorReads, With<Faction>>,
    doors: Query<DoorReads>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<OpenDoorAct>>,
) {
    let actor = (**selected).and_then(|actor| actors.get(actor).ok());
    let target = actor.and_then(|(actor_pos, _)| {
        doors
            .iter()
            .find(|(_, open_state, door_cell)| {
                !*open_state.is_open() && *is_8_adjacent(*actor_pos, Position::new(***door_cell))
            })
            .map(|(entity, ..)| entity)
    });
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, open_door_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

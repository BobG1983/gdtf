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
                    !*open_state.is_open()
                        && *is_8_adjacent(*actor_pos, Position::new(***door_cell))
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::EnterEmplacementAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    emplacement::EmplacementState,
    entity::TerrainCell,
    ganger::{Faction, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Contextual button that mounts an adjacent emplacement.
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

type EnterEmplacementReads = (Entity, &'static EmplacementState, &'static TerrainCell);

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
                    !*state.is_occupied()
                        && *is_8_adjacent(*actor_pos, Position::new(***emplacement_cell))
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

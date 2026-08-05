use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExitEmplacementAct};
use gdtf_battle_sim::{
    emplacement::{EmplacementOccupant, EmplacementState},
    ganger::{Faction, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Contextual button that dismounts the occupied emplacement.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExitEmplacementButton;
}

impl ContextualPanelAct for ExitEmplacementAct {
    type Marker = ExitEmplacementButton;

    const SLOT: PanelSlot = PanelSlot::new(6);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Exit")
    }
}

pub(in crate::states::running::game::battlescape) fn offer_exit_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<(), (With<Position>, With<Faction>)>,
    emplacements: Query<(Entity, &EmplacementOccupant), With<EmplacementState>>,
    mut offer: ResMut<ContextualOffer<ExitEmplacementAct>>,
) {
    let target = (**selected)
        .filter(|actor| actors.get(*actor).is_ok())
        .and_then(|actor| {
            emplacements
                .iter()
                .find(|(_, occupant)| ***occupant == actor)
                .map(|(entity, _)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

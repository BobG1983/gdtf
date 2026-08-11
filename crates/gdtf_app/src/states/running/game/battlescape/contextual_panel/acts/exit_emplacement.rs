use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExitEmplacementAct};
use gdtf_battle_sim::{
    acts::exit_emplacement_tu_cost,
    emplacement::{EmplacementOccupant, EmplacementState},
    ganger::{Faction, Position, Tu},
    tu::can_spend_tu,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
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

type ExitEmplacementActorFilter = (With<Position>, With<Faction>);

pub(in crate::states::running::game::battlescape) fn offer_exit_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<Option<&Tu>, ExitEmplacementActorFilter>,
    emplacements: Query<(Entity, &EmplacementOccupant), With<EmplacementState>>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<ExitEmplacementAct>>,
) {
    let actor = (**selected).and_then(|actor| actors.get(actor).ok().map(|tu| (actor, tu)));
    let target = actor.and_then(|(actor, _)| {
        emplacements
            .iter()
            .find(|(_, occupant)| ***occupant == actor)
            .map(|(entity, _)| entity)
    });
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, exit_emplacement_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

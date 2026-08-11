use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::EnterEmplacementAct};
use gdtf_battle_sim::{
    acts::{downed::is_8_adjacent, enter_emplacement_tu_cost},
    emplacement::EmplacementState,
    entity::TerrainCell,
    ganger::{Faction, Position, Tu},
    tu::can_spend_tu,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
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

type EnterEmplacementActorReads = (&'static Position, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_enter_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<EnterEmplacementActorReads, With<Faction>>,
    emplacements: Query<EnterEmplacementReads>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<EnterEmplacementAct>>,
) {
    let actor = (**selected).and_then(|actor| actors.get(actor).ok());
    let target = actor.and_then(|(actor_pos, _)| {
        emplacements
            .iter()
            .find(|(_, state, emplacement_cell)| {
                !*state.is_occupied()
                    && *is_8_adjacent(*actor_pos, Position::new(***emplacement_cell))
            })
            .map(|(entity, ..)| entity)
    });
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, enter_emplacement_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ShoveAct};
use gdtf_battle_sim::{
    acts::{downed::is_8_adjacent, shove_tu_cost},
    ganger::{Faction, LifeState, Position, Tu},
    tu::can_spend_tu,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that shoves an adjacent ganger.
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

type ShoveCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
);

type ShoveActorReads = (&'static Position, &'static Faction, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_shove(
    selected: Res<SelectedShooter>,
    actors: Query<ShoveActorReads>,
    candidates: Query<ShoveCandidates>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<ShoveAct>>,
) {
    let actor = (**selected).and_then(|actor| actors.get(actor).ok());
    let target = actor.and_then(|(actor_pos, actor_faction, _)| {
        candidates
            .iter()
            .find(|(_, pos, life, faction)| {
                **life == LifeState::Alive
                    && **faction != *actor_faction
                    && *is_8_adjacent(*actor_pos, **pos)
            })
            .map(|(entity, ..)| entity)
    });
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, _, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, shove_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::StabilizeAct};
use gdtf_battle_sim::{
    acts::downed::{is_8_adjacent, stabilize_tu_cost},
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position, Tu},
    tu::can_spend_tu,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that stabilizes an adjacent bleeding ganger.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StabilizeButton;
}

impl ContextualPanelAct for StabilizeAct {
    type Marker = StabilizeButton;

    const SLOT: PanelSlot = PanelSlot::new(1);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Stabilize")
    }
}

type StabilizeCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static BleedingOut>,
);

type StabilizeActorReads = (&'static Position, &'static Faction, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_stabilize(
    selected: Res<SelectedShooter>,
    actors: Query<StabilizeActorReads>,
    candidates: Query<StabilizeCandidates>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<StabilizeAct>>,
) {
    let actor = (**selected).and_then(|actor| actors.get(actor).ok());
    let target = actor.and_then(|(actor_pos, actor_faction, _)| {
        candidates
            .iter()
            .find(|(_, pos, life, faction, bleeding_out)| {
                **life == LifeState::Downed
                    && **faction == *actor_faction
                    && bleeding_out.is_some()
                    && *is_8_adjacent(*actor_pos, **pos)
            })
            .map(|(entity, ..)| entity)
    });
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, _, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, stabilize_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

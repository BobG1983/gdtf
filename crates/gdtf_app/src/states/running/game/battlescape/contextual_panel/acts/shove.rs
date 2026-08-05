use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ShoveAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
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

pub(in crate::states::running::game::battlescape) fn offer_shove(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<ShoveCandidates>,
    mut offer: ResMut<ContextualOffer<ShoveAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
            candidates
                .iter()
                .find(|(_, pos, life, faction)| {
                    **life == LifeState::Alive
                        && **faction != *actor_faction
                        && *is_8_adjacent(*actor_pos, **pos)
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

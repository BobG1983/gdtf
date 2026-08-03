use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExecuteAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExecuteButton;
}

impl ContextualPanelAct for ExecuteAct {
    type Marker = ExecuteButton;

    const SLOT: PanelSlot = PanelSlot::new(0);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Execute")
    }
}

type ExecuteCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
);

pub(in crate::states::running::game::battlescape) fn offer_execute(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<ExecuteCandidates>,
    mut offer: ResMut<ContextualOffer<ExecuteAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
            candidates
                .iter()
                .find(|(_, pos, life, faction)| {
                    **life == LifeState::Downed
                        && **faction != *actor_faction
                        && *is_8_adjacent(*actor_pos, **pos)
                })
                .map(|(entity, ..)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

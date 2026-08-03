use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::StabilizeAct};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
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

pub(in crate::states::running::game::battlescape) fn offer_stabilize(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Faction)>,
    candidates: Query<StabilizeCandidates>,
    mut offer: ResMut<ContextualOffer<StabilizeAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction)| {
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
    offer.set_if_neq(ContextualOffer::new(target));
}

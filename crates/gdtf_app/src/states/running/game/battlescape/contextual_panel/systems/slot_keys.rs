use bevy::prelude::*;
use gdtf_battle_input::{Keybinds, SlotRank, contextual::PendingContextualIntents};

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualActButton, ContextualOffer, ContextualPanelAct, PanelSlot, VisibleSlotRank,
};

pub(in crate::states::running::game::battlescape) fn rank_visible_contextual_buttons(
    mut buttons: Query<(
        Entity,
        &Visibility,
        &ContextualActButton,
        &mut VisibleSlotRank,
    )>,
) {
    let mut visible: Vec<(PanelSlot, Entity)> = buttons
        .iter()
        .filter(|(_, visibility, ..)| **visibility == Visibility::Visible)
        .map(|(entity, _, button, _)| (**button, entity))
        .collect();
    visible.sort_by_key(|(slot, entity)| (*slot, *entity));

    for (entity, _, _, mut rank) in &mut buttons {
        let want = VisibleSlotRank::new(
            visible
                .iter()
                .position(|(_, candidate)| *candidate == entity)
                .and_then(|index| u8::try_from(index + 1).ok().map(SlotRank::new)),
        );
        if *rank != want {
            *rank = want;
        }
    }
}

pub(in crate::states::running::game::battlescape) fn press_contextual_button_via_key<
    A: ContextualPanelAct,
>(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    offer: Res<ContextualOffer<A>>,
    ranks: Query<&VisibleSlotRank, With<A::Marker>>,
    mut pending: ResMut<PendingContextualIntents<A>>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Ok(visible_rank) = ranks.single() else {
        return;
    };
    let Some(rank) = visible_rank.rank() else {
        return;
    };
    let Some(key) = Keybinds::contextual_slot_key(rank) else {
        return;
    };
    if keys.just_pressed(key)
        && let Some(target) = offer.target()
    {
        pending.push(target);
    }
}

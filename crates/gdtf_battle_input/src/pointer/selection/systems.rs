//! Left-click and right-click selection systems.

use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::{
    ActIntent, PendingActIntent,
    fire_surface::ShooterArms,
    selection::{
        decision::{
            LeftClickReads, PointerSelection, TurnReads, apply_left_click, apply_pin,
            decide_left_click, decide_pin, decide_turn,
        },
        resources::SelectedShooter,
    },
};

/// Handle left-click: decide act, apply selection, and update inspect pin.
pub fn left_click_act(
    gate: PlaybackGate,
    reads: LeftClickReads,
    factions: Query<&Faction>,
    lifes: Query<&LifeState>,
    arms: ShooterArms,
    mut selection: PointerSelection,
    mut pending: ResMut<PendingActIntent>,
) {
    if !reads.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let gate_open = gate.is_open();
    let outcome =
        gate_open.then(|| decide_left_click(&reads, &selection, &factions, &lifes, &arms));
    let pin = decide_pin(&reads, &selection.inspect, &factions);
    if let Some(outcome) = outcome {
        apply_left_click(outcome, &mut selection, &mut pending);
    }
    apply_pin(pin, &mut selection);
}

/// Handle right-click: queue a turn-to-face intent when a player ganger is selected.
pub fn right_click_turn_to_face(
    mouse: Res<ButtonInput<MouseButton>>,
    reads: TurnReads,
    factions: Query<&Faction>,
    positions: Query<&Position>,
    mut pending: ResMut<PendingActIntent>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    if !selection_is_player(*reads.selected, &factions, *reads.player) {
        return;
    }
    if let Some(request) = decide_turn(&reads.selected, &reads.hovered, &positions) {
        pending.push(ActIntent::Turn(request));
    }
}

fn selection_is_player(
    selected: SelectedShooter,
    factions: &Query<&Faction>,
    player: PlayerFaction,
) -> bool {
    (*selected)
        .and_then(|actor| factions.get(actor).ok().copied())
        .is_some_and(|faction| faction == *player)
}

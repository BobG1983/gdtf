//! Left-click and right-click selection systems.

use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    fire::MeleeQuery,
    ganger::LifeState,
    prelude::{Faction, Position},
    weapon::{WieldedBy, Wields},
};

use crate::{
    ActIntent, InspectTarget, PendingActIntent,
    fire_surface::{ShooterFireData, WeaponMagazine},
    selection::{
        PathPreviewTarget,
        decision::{
            LeftClickReads, TurnReads, apply_left_click, apply_pin, decide_left_click, decide_pin,
            decide_turn,
        },
        resources::SelectedShooter,
    },
};

#[expect(
    clippy::too_many_arguments,
    reason = "needs Wields, magazine, melee probe, and PathPreviewTarget; LeftClickReads already bundles Res reads"
)]
/// Handle left-click: decide act, apply selection, and update inspect pin.
pub fn left_click_act(
    gate: PlaybackGate,
    reads: LeftClickReads,
    factions: Query<&Faction>,
    lifes: Query<&LifeState>,
    shooters: Query<ShooterFireData>,
    wields: Query<&Wields>,
    weapons: Query<WeaponMagazine, With<WieldedBy>>,
    melee: MeleeQuery,
    mut selected: ResMut<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
    mut inspect: ResMut<InspectTarget>,
    mut target: ResMut<PathPreviewTarget>,
) {
    if !reads.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let gate_open = gate.is_open();
    let outcome = gate_open.then(|| {
        decide_left_click(
            &reads, &inspect, &target, &factions, &lifes, &shooters, &wields, &weapons, &melee,
            &selected,
        )
    });
    let pin = decide_pin(&reads, &inspect, &factions);
    if let Some(outcome) = outcome {
        apply_left_click(outcome, &mut selected, &mut pending, &mut target);
    }
    apply_pin(pin, &mut inspect);
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

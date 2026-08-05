//! Gamepad cursor movement and click/turn systems.

use bevy::{input::gamepad::Gamepad, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::GamepadCursorMoved;
use gdtf_battle_sim::{
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::{
    ActIntent, PendingActIntent,
    fire_surface::ShooterArms,
    gamepad::cursor::{
        ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, GamepadCursor, move_cursor,
    },
    selection::{
        LeftClickReads, PointerSelection, TurnReads, apply_left_click, apply_pin,
        decide_left_click, decide_pin, decide_turn,
    },
};

/// Move the gamepad cursor from left stick and claim pointer ownership.
pub fn move_gamepad_cursor(
    gamepads: Query<&Gamepad>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut cursor: ResMut<GamepadCursor>,
    mut active: ResMut<ActivePointer>,
) {
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    let stick = gamepad.left_stick();
    let next = move_cursor(
        **cursor,
        stick,
        CURSOR_SPEED,
        time.delta_secs(),
        window.size(),
    );
    if **cursor != next {
        *cursor = GamepadCursor::new(next);
    }
    if stick.length() > *CURSOR_STICK_DEADZONE && *active != ActivePointer::Gamepad {
        *active = ActivePointer::Gamepad;
    }
}

/// Return pointer ownership to the mouse when it moves.
pub fn mouse_reclaims_pointer(
    mut moves: MessageReader<bevy::window::CursorMoved>,
    mut active: ResMut<ActivePointer>,
) {
    if moves.read().next().is_some() && *active != ActivePointer::Mouse {
        *active = ActivePointer::Mouse;
    }
}

/// South button: same left-click decision path as mouse.
pub fn gamepad_click_act(
    gamepads: Query<&Gamepad>,
    reads: LeftClickReads,
    factions: Query<&Faction>,
    lifes: Query<&LifeState>,
    arms: ShooterArms,
    mut selection: PointerSelection,
    mut pending: ResMut<PendingActIntent>,
) {
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };
    if !gamepad.just_pressed(GamepadButton::South) {
        return;
    }
    let outcome = decide_left_click(&reads, &selection, &factions, &lifes, &arms);
    let pin = decide_pin(&reads, &selection.inspect, &factions);
    apply_left_click(outcome, &mut selection, &mut pending);
    apply_pin(pin, &mut selection);
}

/// East button: turn-to-face like right-click.
pub fn gamepad_turn(
    gamepads: Query<&Gamepad>,
    reads: TurnReads,
    factions: Query<&Faction>,
    positions: Query<&Position>,
    mut pending: ResMut<PendingActIntent>,
) {
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };
    if !gamepad.just_pressed(GamepadButton::East) {
        return;
    }
    let selection_player = (**reads.selected)
        .and_then(|actor| factions.get(actor).ok().copied())
        .is_some_and(|faction| faction == **reads.player);
    if !selection_player {
        return;
    }
    if let Some(request) = decide_turn(&reads.selected, &reads.hovered, &positions) {
        pending.push(ActIntent::Turn(request));
    }
}

/// Emit cursor-moved messages while the gamepad owns the pointer.
pub fn emit_gamepad_cursor_move(
    active: Res<ActivePointer>,
    cursor: Res<GamepadCursor>,
    mut moves: MessageWriter<GamepadCursorMoved>,
) {
    if *active == ActivePointer::Gamepad {
        moves.write(GamepadCursorMoved::new(**cursor));
    }
}

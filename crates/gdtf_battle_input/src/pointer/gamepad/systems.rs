//! The gamepad input systems (GTW-259): the software-cursor drive, the pointer arbitration, the
//! South / East act surfaces, and the edge-pan emitter.

use bevy::{input::gamepad::Gamepad, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::GamepadCursorMoved;
use gdtf_battle_sim::{
    fire::MeleeQuery,
    prelude::{Faction, Position},
    weapon::{WieldedBy, Wields},
};

use crate::{
    ActIntent, InspectTarget, PendingActIntent,
    fire_surface::{ShooterFireData, WeaponMagazine},
    gamepad::cursor::{
        ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, GamepadCursor, move_cursor,
    },
    selection::{
        LeftClickReads, PathPreviewTarget, SelectedShooter, TurnReads, apply_left_click, apply_pin,
        decide_left_click, decide_pin, decide_turn,
    },
};

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(pick_hovered_cell)`): moves the
/// [`GamepadCursor`] by the LEFT stick and claims the [`ActivePointer`] for the gamepad when the
/// stick passes the deadzone (GTW-259).
///
/// Reads the first connected [`Gamepad`]'s [`left_stick`](Gamepad::left_stick), the primary
/// [`Window`] size, and `Res<Time>` ([`delta_secs`](Time::delta_secs)); computes the new cursor
/// via the pure [`move_cursor`] (the y-flip + window clamp); writes the [`GamepadCursor`]; and —
/// only when the stick magnitude exceeds [`CURSOR_STICK_DEADZONE`] — sets [`ActivePointer::Gamepad`]
/// (so a resting stick never steals the pointer from the mouse). With no gamepad or no window it
/// leaves both resources untouched.
///
/// Ordered `.before(pick_hovered_cell)` (`bevy-traps.md` #3) so the picker projects THIS update's
/// cursor when the gamepad is active. Param-only (`Query` / `Res` / `ResMut`), no `&mut World`.
///
/// HONESTY: the raw [`Gamepad`] stick read is device-event-driven and not headlessly drivable; the
/// pure [`move_cursor`] math + in-engine QA cover it (the module-level note).
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
    // Only CLAIM the pointer when the stick is genuinely deflected (past the deadzone) — a resting
    // stick must not steal control from the mouse (last-moved-wins).
    if stick.length() > *CURSOR_STICK_DEADZONE && *active != ActivePointer::Gamepad {
        *active = ActivePointer::Gamepad;
    }
}

/// `Update` (battle-gated, `InputSystems::Gather`): the MOUSE reclaims the pointer when the OS
/// cursor moves (GTW-259 AC4, last-moved-wins).
///
/// Drains the [`MessageReader<CursorMoved>`](bevy::window::CursorMoved) (the OS cursor-move
/// message, `bevy-traps.md` #4); any message this update means the mouse moved, so it sets
/// [`ActivePointer::Mouse`]. No message → the active pointer is unchanged.
///
/// Param-only (`MessageReader` / `ResMut`), no `&mut World` (`bevy-traps.md` #7).
pub fn mouse_reclaims_pointer(
    mut moves: MessageReader<bevy::window::CursorMoved>,
    mut active: ResMut<ActivePointer>,
) {
    if moves.read().next().is_some() && *active != ActivePointer::Mouse {
        *active = ActivePointer::Mouse;
    }
}

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(dispatch_act_intents)`): South =
/// left-click equivalent — resolves the SHARED left-click decision on a South just-press (GTW-259).
///
/// On a `GamepadButton::South` just-press (read off the first [`Gamepad`] component, the
/// `gdtf_ui::focus_nav` GTW-119 precedent) it runs the SAME
/// [`decide_left_click`](crate::selection::decide_left_click) /
/// [`apply_left_click`](crate::selection::apply_left_click) the mouse's
/// [`left_click_act`](crate::selection::left_click_act) uses — so South FIRES / SELECTS / MOVES /
/// CLEARS through the identical precedence (no new `ActIntent` variant; the ONE
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits the carried act). With no
/// gamepad, or no South press, nothing happens.
///
/// GTW-300 — South ALSO resolves the PARALLEL inspect-panel pin
/// ([`decide_pin`] / [`apply_pin`]) for free, since it routes through the SAME shared decision:
/// clicking cover / an enemy PINS the panel, an empty tile UNPINS, a SELECT / FIRE keeps the pin.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + read-only
/// `Query<&Faction>` / `Query<ShooterFireData>` / `Query<&Wields>` + the weapon-magazine query +
/// the [`MeleeWeapon`](gdtf_battle_sim::weapon::MeleeWeapon) marker probe ([`MeleeQuery`] — the fire
/// guard's magazine lives on the related RANGED weapon
/// entity since GTW-323 slice 3, resolved excluding the melee weapon since GTW-505 C5) + the
/// [`ResMut<SelectedShooter>`] / [`ResMut<PendingActIntent>`] / [`ResMut<InspectTarget>`] /
/// [`ResMut<PathPreviewTarget>`](crate::selection::PathPreviewTarget) (GTW-356 two-click target)
/// writes + the [`Gamepad`] query; no `&mut World`. Runs `.before(pick_hovered_cell)` and
/// `.before(dispatch_act_intents)`.
///
/// HONESTY: the raw South-button read is device-event-driven and not headlessly drivable; the
/// shared decision (exercised via the mouse path) + in-engine QA cover it.
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-323 slice 3: mirrors left_click_act — the fire guard's magazine moved to the \
              related weapon entity, so the shared decision needs the extra Wields + weapon-magazine \
              queries on top of the gamepad + reads/writes; GTW-356 adds the PathPreviewTarget write; \
              GTW-505 C5 adds the MeleeWeapon marker probe for ranged-weapon resolution"
)]
pub fn gamepad_click_act(
    gamepads: Query<&Gamepad>,
    reads: LeftClickReads,
    factions: Query<&Faction>,
    shooters: Query<ShooterFireData>,
    wields: Query<&Wields>,
    weapons: Query<WeaponMagazine, With<WieldedBy>>,
    melee: MeleeQuery,
    mut selected: ResMut<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
    mut inspect: ResMut<InspectTarget>,
    mut target: ResMut<PathPreviewTarget>,
) {
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };
    if !gamepad.just_pressed(GamepadButton::South) {
        return;
    }
    // Decide both effects from the immutable inspect + move-target reads FIRST, then commit them
    // (the pin / target writes borrow `inspect` / `target` mutably, so the read-only decisions
    // must finish before them — the GTW-300 InspectTarget precedent + the GTW-356 two-click
    // PathPreviewTarget state machine).
    let outcome = decide_left_click(
        &reads, &inspect, &target, &factions, &shooters, &wields, &weapons, &melee, &selected,
    );
    let pin = decide_pin(&reads, &inspect, &factions);
    // GTW-356 — the SAME two-click move-target commit the mouse path runs (set/commit/clear of
    // PathPreviewTarget); South shares the decision, so the two-click flow is identical.
    apply_left_click(outcome, &mut selected, &mut pending, &mut target);
    // GTW-300 — the parallel pin effect (same shared decision as the mouse path).
    apply_pin(pin, &mut inspect);
}

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(dispatch_act_intents)`): East =
/// right-click equivalent — turns the player-faction selection to face the cursor on an East
/// just-press (GTW-259).
///
/// On a `GamepadButton::East` just-press (the [`Gamepad`] component, the GTW-119 precedent) with a
/// player-faction [`SelectedShooter`] it runs the SAME
/// [`decide_turn`](crate::selection::decide_turn) the mouse's
/// [`right_click_turn_to_face`](crate::selection::right_click_turn_to_face) uses and pushes the
/// resulting [`ActIntent::Turn`] (no new variant). The player-faction gate (a forced enemy
/// selection emits nothing) is applied here. With no gamepad, no East press, a non-player
/// selection, or a `None` decision, nothing happens.
///
/// Param-only (`bevy-traps.md` #7): read-only `Query<&Gamepad>` / `Query<&Faction>` /
/// `Query<&Position>` + `Res<PlayerFaction>` / `Res<SelectedShooter>` / `Res<InspectTarget>` + the
/// [`ResMut<PendingActIntent>`] write; no `&mut World`. Runs `.before(dispatch_act_intents)`.
///
/// HONESTY: the raw East-button read is device-event-driven and not headlessly drivable; the shared
/// decision (exercised via the mouse path) + in-engine QA cover it.
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
    // Gating: only a PLAYER-faction selection turns (a forced enemy emits nothing).
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

/// `Update` (battle-gated, `InputSystems::Gather`): emit the [`GamepadCursor`]'s screen position
/// for the presenter's edge-pan when the gamepad is the active pointer (GTW-259).
///
/// When [`ActivePointer::Gamepad`], writes one [`GamepadCursorMoved`]`(`[`*cursor`](GamepadCursor)`)`
/// message (the presenter-defined edge-pan input API) so the presenter's
/// `pan_camera_on_gamepad_cursor_edge` can REUSE the GTW-250 mouse-edge pan for the gamepad cursor.
/// In [`ActivePointer::Mouse`] mode it emits nothing — GTW-250's mouse-edge pan already covers the
/// OS cursor, so this never double-pans.
///
/// Param-only (`bevy-traps.md` #7): a `Res<ActivePointer>` + `Res<GamepadCursor>` read and a
/// [`MessageWriter<GamepadCursorMoved>`](bevy::ecs::message::MessageWriter) write (the
/// input→presenter edge — input names a presenter-defined message, the `HighlightRequest`
/// precedent); no `&mut World`.
pub fn emit_gamepad_cursor_move(
    active: Res<ActivePointer>,
    cursor: Res<GamepadCursor>,
    mut moves: MessageWriter<GamepadCursorMoved>,
) {
    if *active == ActivePointer::Gamepad {
        moves.write(GamepadCursorMoved::new(**cursor));
    }
}

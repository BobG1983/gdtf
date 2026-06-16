//! Gamepad software cursor (GTW-259): a stick-driven screen cursor that drives the SAME
//! select / move / fire / turn path the mouse does, so a gamepad can play the battle.
//!
//! Fixes play-test bug #7 (gamepad half): battle control was mouse-only. This module adds a
//! SOFTWARE cursor steered by the gamepad LEFT stick that:
//!
//! 1. moves a [`GamepadCursor`] screen position each update by the stick × [`CursorSpeed`] ×
//!    `dt` (the y-FLIPped [`move_cursor`] pure helper), claiming the [`ActivePointer`] for
//!    the gamepad whenever the stick passes the [`CursorStickDeadzone`];
//! 2. lets the generalized picker ([`pick_hovered_cell`](crate::pick_hovered_cell)) project
//!    THAT cursor (instead of the OS cursor) when the gamepad is the active pointer — so the
//!    landed GTW-251 highlight follows the gamepad cursor for free (NO separate reticle);
//! 3. resolves South (left-click equivalent) through the SHARED
//!    [`decide_left_click`](crate::selection::decide_left_click) /
//!    [`apply_left_click`](crate::selection::apply_left_click) and East (right-click
//!    equivalent) through the SHARED [`decide_turn`](crate::selection::decide_turn) — ONE
//!    precedence implementation, two devices (the defect this prevents is duplicating the
//!    precedence per device); and
//! 4. emits the cursor's screen position as a presenter-defined [`GamepadCursorMoved`]
//!    message so the presenter can EDGE-PAN (input→presenter, no cycle — the
//!    [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest) precedent).
//!
//! # Mouse + gamepad coexist (last-moved-input wins)
//!
//! [`ActivePointer`] arbitrates: the stick passing the deadzone sets
//! [`ActivePointer::Gamepad`]; a [`CursorMoved`] message (the OS mouse moved) sets
//! [`ActivePointer::Mouse`] ([`mouse_reclaims_pointer`]). The picker reads it to choose which
//! cursor to project, so whichever device moved last drives the highlighted cell.
//!
//! # South / East are hardcoded (the `focus_nav` GTW-119 precedent)
//!
//! South = left-click / act, East = right-click / turn — read directly off the [`Gamepad`]
//! component ([`GamepadButton::South`] / [`GamepadButton::East`]), the same way
//! `gdtf_ui::focus_nav` reads [`GamepadButton::South`] for activation. Gamepad remapping is a
//! future ticket.
//!
//! # Honesty: raw pad state is not headlessly drivable
//!
//! Real [`Gamepad`] stick / button STATE is device-event-driven in Bevy 0.18 and cannot be
//! cleanly driven from a headless test (the same limitation GTW-250 documents). So the raw
//! stick / button READS ([`move_gamepad_cursor`], [`gamepad_click_act`], [`gamepad_turn`])
//! are covered by the pure [`move_cursor`] helper + the shared decision (exercised via the
//! mouse path) + in-engine QA; the headless tests prove the SETTABLE-resource / message logic
//! (the picker arbitration, the shared decision, the edge-pan from a message).

use bevy::{input::gamepad::Gamepad, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::GamepadCursorMoved;
use gdtf_battle_sim::{Faction, PlayerFaction, Position};

use crate::{
    ActIntent, HoveredCell, PendingActIntent,
    fire_surface::ShooterFireData,
    selection::{
        LeftClickReads, SelectedShooter, apply_left_click, decide_left_click, decide_turn,
    },
};

/// The gamepad software cursor's SCREEN position, in logical px (window origin TOP-LEFT,
/// y-down — the same frame the OS cursor lives in).
///
/// A NAMED single-field newtype over [`Vec2`] (no-bare-types: the cursor position is a
/// domain value; the inner [`Vec2`] is the framework-math screen-coordinate carve-out the
/// GTW-249/250/251 helpers already use), [`Deref`]ing to its inner [`Vec2`] so a reader uses
/// it directly. A [`Resource`] initialised to a sensible default
/// ([`Self::DEFAULT_CENTRE`]) until the first window-relative move recentres it.
/// [`move_gamepad_cursor`] writes it; the generalized
/// [`pick_hovered_cell`](crate::pick_hovered_cell) projects it when the gamepad is the active
/// pointer.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursor(pub Vec2);

impl GamepadCursor {
    /// The initial cursor position before the window size is known: a fixed off-origin
    /// screen point so the cursor starts somewhere on a typical window rather than the
    /// top-left corner. The first [`move_gamepad_cursor`] update clamps it into the real
    /// window extent. A framework-math screen coordinate (the same carve-out the type uses).
    pub const DEFAULT_CENTRE: Vec2 = Vec2::new(640.0, 360.0);
}

impl Default for GamepadCursor {
    /// The default cursor position ([`Self::DEFAULT_CENTRE`]).
    fn default() -> Self {
        Self(Self::DEFAULT_CENTRE)
    }
}

/// Which input device last moved — the last-moved-wins arbiter the picker reads to choose
/// which cursor to project (GTW-259).
///
/// A domain enum (no-bare-types: the active pointer is a named control-state value, not a
/// bare discriminant), a [`Resource`] defaulting to [`Mouse`](Self::Mouse) (the mouse-only
/// landed behavior until the stick moves). [`move_gamepad_cursor`] sets
/// [`Gamepad`](Self::Gamepad) when the stick passes the deadzone;
/// [`mouse_reclaims_pointer`] sets [`Mouse`](Self::Mouse) on a [`CursorMoved`] message.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePointer {
    /// The OS mouse is the active pointer — the picker projects `window.cursor_position()`
    /// (the landed path). The default.
    #[default]
    Mouse,
    /// The gamepad software cursor is the active pointer — the picker projects the
    /// [`GamepadCursor`].
    Gamepad,
}

/// The gamepad cursor's travel speed, in screen px per second.
///
/// A domain quantity (no-bare-types — px/sec is a real unit, never a bare `f32`), a newtype
/// over a private [`f32`] with derived [`Deref`]. An INPUT tunable (how fast the stick glides
/// the software cursor), so a doc-commented input-level const ([`CURSOR_SPEED`]), NOT a
/// `.ron` data file — FLAGGED for data-driving if the user later wants it authored /
/// hot-swappable (the GTW-250 [`PanSpeed`](gdtf_battle_presenter::PanSpeed) view-const
/// precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorSpeed(f32);

impl CursorSpeed {
    /// Construct a [`CursorSpeed`] from screen px-per-second.
    #[must_use]
    pub const fn new(px_per_second: f32) -> Self {
        Self(px_per_second)
    }
}

/// The left-stick deadzone: a stick magnitude at or below this contributes no cursor move and
/// does NOT claim the pointer for the gamepad.
///
/// A unitless `[0, 1]` analog-stick magnitude threshold (no-bare-types — a newtype over a
/// private [`f32`] with derived [`Deref`]). An INPUT tunable (a doc-commented const
/// [`CURSOR_STICK_DEADZONE`], not `.ron`; FLAGGED for data-driving) so a resting stick never
/// drifts the cursor or steals the pointer from the mouse (the GTW-250
/// [`StickDeadzone`](gdtf_battle_presenter::StickDeadzone) precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorStickDeadzone(f32);

impl CursorStickDeadzone {
    /// Construct a [`CursorStickDeadzone`] from a unitless `[0, 1]` magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The shipping gamepad-cursor speed: screen px the software cursor glides per second at full
/// stick (see [`CursorSpeed`]).
///
/// An input const. Chosen so a full-stick sweep crosses a typical window in a second or two;
/// FLAGGED for data-driving later.
pub const CURSOR_SPEED: CursorSpeed = CursorSpeed::new(900.0);

/// The shipping gamepad left-stick deadzone (see [`CursorStickDeadzone`]): a stick magnitude
/// at or below this is ignored so a resting stick never drifts the cursor or claims the
/// pointer.
pub const CURSOR_STICK_DEADZONE: CursorStickDeadzone = CursorStickDeadzone::new(0.15);

/// The new screen position of a software cursor at `pos` after the LEFT stick moves it for
/// `dt` seconds at `speed`, with the stick-y → screen-y flip and a clamp inside the window.
///
/// GTW-259 AC1 — pure, total, unit-tested. The window's origin is TOP-LEFT and screen-y grows
/// DOWNWARD, while a gamepad stick reports UP as `+y`; so a stick pushed UP (`stick.y > 0`)
/// must move the cursor UP on screen (screen-y DECREASES) — the y is FLIPPED:
/// `new = pos + (stick.x, -stick.y) * speed * dt`. The x needs no flip (stick right `+x` →
/// cursor right `+x`). The result is clamped componentwise into `[Vec2::ZERO, window]` so the
/// cursor can never leave the window. A zero stick leaves `pos` unchanged (clamped, so a
/// pos already inside the window is returned as-is).
///
/// Pure / total — no `App`, no `World`, no side effects. `Vec2` is framework-math plumbing
/// (the GTW-249/250/251 carve-out for raw screen / direction coords), not a domain newtype.
#[must_use]
pub fn move_cursor(pos: Vec2, stick: Vec2, speed: CursorSpeed, dt: f32, window: Vec2) -> Vec2 {
    // Stick-y → screen-y FLIP: stick UP (+y) moves the cursor UP (screen -y).
    let delta = Vec2::new(stick.x, -stick.y) * *speed * dt;
    let moved = pos + delta;
    // Clamp componentwise inside the window so the cursor can never leave it.
    moved.clamp(Vec2::ZERO, window)
}

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(pick_hovered_cell)`): moves the
/// [`GamepadCursor`] by the LEFT stick and claims the [`ActivePointer`] for the gamepad when
/// the stick passes the deadzone (GTW-259).
///
/// Reads the first connected [`Gamepad`]'s [`left_stick`](Gamepad::left_stick), the primary
/// [`Window`] size, and `Res<Time>` ([`delta_secs`](Time::delta_secs)); computes the new
/// cursor via the pure [`move_cursor`] (the y-flip + window clamp); writes the
/// [`GamepadCursor`]; and — only when the stick magnitude exceeds [`CURSOR_STICK_DEADZONE`] —
/// sets [`ActivePointer::Gamepad`] (so a resting stick never steals the pointer from the
/// mouse). With no gamepad or no window it leaves both resources untouched.
///
/// Ordered `.before(pick_hovered_cell)` (`bevy-traps.md` #3) so the picker projects THIS
/// update's cursor when the gamepad is active. Param-only (`Query` / `Res` / `ResMut`), no
/// `&mut World` (`bevy-traps.md` #7).
///
/// HONESTY: the raw [`Gamepad`] stick read is device-event-driven and not headlessly
/// drivable; the pure [`move_cursor`] math + in-engine QA cover it (the module-level note).
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
        *cursor = GamepadCursor(next);
    }
    // Only CLAIM the pointer when the stick is genuinely deflected (past the deadzone) — a
    // resting stick must not steal control from the mouse (last-moved-wins).
    if stick.length() > *CURSOR_STICK_DEADZONE && *active != ActivePointer::Gamepad {
        *active = ActivePointer::Gamepad;
    }
}

/// `Update` (battle-gated, `InputSystems::Gather`): the MOUSE reclaims the pointer when the
/// OS cursor moves (GTW-259 AC4, last-moved-wins).
///
/// Drains the [`MessageReader<CursorMoved>`] (Bevy 0.18 — the OS cursor-move message,
/// `bevy-traps.md` #4); any message this update means the mouse moved, so it sets
/// [`ActivePointer::Mouse`] (the picker then projects the OS cursor again). No message → the
/// active pointer is unchanged (the gamepad keeps control if it had it).
///
/// Param-only (`MessageReader` / `ResMut`), no `&mut World` (`bevy-traps.md` #7).
pub fn mouse_reclaims_pointer(
    mut moves: MessageReader<CursorMoved>,
    mut active: ResMut<ActivePointer>,
) {
    if moves.read().next().is_some() && *active != ActivePointer::Mouse {
        *active = ActivePointer::Mouse;
    }
}

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(dispatch_act_intents)`): South =
/// left-click equivalent — resolves the SHARED left-click decision on a South just-press
/// (GTW-259).
///
/// On a `GamepadButton::South` just-press (read off the first [`Gamepad`] component, the
/// `gdtf_ui::focus_nav` GTW-119 precedent) it runs the SAME
/// [`decide_left_click`](crate::selection::decide_left_click) /
/// [`apply_left_click`](crate::selection::apply_left_click) the mouse's
/// [`left_click_act`](crate::selection::left_click_act) uses — so South FIRES / SELECTS /
/// MOVES / CLEARS through the identical FIRE → SELECT → MOVE → CLEAR precedence (no new
/// `ActIntent` variant; the ONE [`dispatch_act_intents`](crate::dispatch_act_intents) drain
/// emits the carried act). With no gamepad, or no South press, nothing happens.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + read-only
/// `Query<&Faction>` / `Query<ShooterFireData>` + the [`ResMut<SelectedShooter>`] /
/// [`ResMut<PendingActIntent>`] writes + the [`Gamepad`] query; no `&mut World`. Runs
/// `.before(pick_hovered_cell)` (the cell resolved last update, the
/// [`left_click_act`](crate::selection::left_click_act) precedent) and
/// `.before(dispatch_act_intents)` (the drain).
///
/// HONESTY: the raw South-button read is device-event-driven and not headlessly drivable; the
/// shared decision (exercised via the mouse path) + in-engine QA cover it (the module note).
pub fn gamepad_click_act(
    gamepads: Query<&Gamepad>,
    reads: LeftClickReads,
    factions: Query<&Faction>,
    shooters: Query<ShooterFireData>,
    mut selected: ResMut<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
) {
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };
    if !gamepad.just_pressed(GamepadButton::South) {
        return;
    }
    let outcome = decide_left_click(&reads, &factions, &shooters, &selected);
    apply_left_click(outcome, &mut selected, &mut pending);
}

/// `Update` (battle-gated, `InputSystems::Gather`, `.before(dispatch_act_intents)`): East =
/// right-click equivalent — turns the player-faction selection to face the cursor on an East
/// just-press (GTW-259).
///
/// On a `GamepadButton::East` just-press (the [`Gamepad`] component, the GTW-119 precedent)
/// with a player-faction [`SelectedShooter`] it runs the SAME
/// [`decide_turn`](crate::selection::decide_turn) the mouse's
/// [`right_click_turn_to_face`](crate::selection::right_click_turn_to_face) uses and pushes
/// the resulting [`ActIntent::Turn`] (no new variant; the ONE
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits it). The player-faction
/// gate (a forced enemy selection emits nothing) is applied here, exactly as the mouse turn
/// surface does. With no gamepad, no East press, a non-player selection, or a `None` decision
/// (nothing hovered / own cell), nothing happens.
///
/// Param-only (`bevy-traps.md` #7): read-only `Query<&Gamepad>` / `Query<&Faction>` /
/// `Query<&Position>` + `Res<PlayerFaction>` / `Res<SelectedShooter>` / `Res<HoveredCell>` +
/// the [`ResMut<PendingActIntent>`] write; no `&mut World`. Runs
/// `.before(dispatch_act_intents)`.
///
/// HONESTY: the raw East-button read is device-event-driven and not headlessly drivable; the
/// shared decision (exercised via the mouse path) + in-engine QA cover it (the module note).
pub fn gamepad_turn(
    gamepads: Query<&Gamepad>,
    hovered: Res<HoveredCell>,
    player: Res<PlayerFaction>,
    selected: Res<SelectedShooter>,
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
    let selection_player = (**selected)
        .and_then(|actor| factions.get(actor).ok().copied())
        .is_some_and(|faction| faction == **player);
    if !selection_player {
        return;
    }
    if let Some(request) = decide_turn(&selected, &hovered, &positions) {
        pending.push(ActIntent::Turn(request));
    }
}

/// `Update` (battle-gated, `InputSystems::Gather`): emit the [`GamepadCursor`]'s screen
/// position for the presenter's edge-pan when the gamepad is the active pointer (GTW-259).
///
/// When [`ActivePointer::Gamepad`], writes one
/// [`GamepadCursorMoved`]`(`[`*cursor`](GamepadCursor)`)` message (the presenter-defined edge-
/// pan input API) so the presenter's
/// [`pan_camera_on_gamepad_cursor_edge`](gdtf_battle_presenter::pan_camera_on_gamepad_cursor_edge)
/// can REUSE the GTW-250 mouse-edge pan for the gamepad cursor. In
/// [`ActivePointer::Mouse`] mode it emits nothing — GTW-250's mouse-edge pan already covers
/// the OS cursor, so this never double-pans.
///
/// Param-only (`bevy-traps.md` #7): a `Res<ActivePointer>` + `Res<GamepadCursor>` read and a
/// [`MessageWriter<GamepadCursorMoved>`](bevy::ecs::message::MessageWriter) write (the input→
/// presenter edge — input names a presenter-defined message, the
/// [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest) precedent); no `&mut World`.
pub fn emit_gamepad_cursor_move(
    active: Res<ActivePointer>,
    cursor: Res<GamepadCursor>,
    mut moves: MessageWriter<GamepadCursorMoved>,
) {
    if *active == ActivePointer::Gamepad {
        moves.write(GamepadCursorMoved(**cursor));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test window size for the cursor clamp.
    const WINDOW: Vec2 = Vec2::new(800.0, 600.0);
    /// A test cursor speed (px/sec).
    const SPEED: CursorSpeed = CursorSpeed::new(100.0);

    /// AC1 — `move_cursor` moves with the stick-y → screen-y FLIP, scales by speed·dt, and a
    /// zero stick leaves the position unchanged.
    #[test]
    fn move_cursor_flips_y_and_scales() {
        let start = Vec2::new(400.0, 300.0); // window centre.

        // Stick UP (+y) moves the cursor UP on screen (screen-y DECREASES) — the flip.
        let up = move_cursor(start, Vec2::new(0.0, 1.0), SPEED, 0.5, WINDOW);
        assert!(
            up.y < start.y,
            "stick UP (+y) must move the cursor UP on screen (screen-y decreases): {} -> {}",
            start.y,
            up.y,
        );
        assert_eq!(
            up.x.to_bits(),
            start.x.to_bits(),
            "a pure-up stick has no x movement",
        );

        // Stick RIGHT (+x) moves the cursor RIGHT (+x), no flip.
        let right = move_cursor(start, Vec2::new(1.0, 0.0), SPEED, 0.5, WINDOW);
        assert!(
            right.x > start.x,
            "stick RIGHT (+x) must move the cursor RIGHT (+x): {} -> {}",
            start.x,
            right.x,
        );

        // speed·dt scales the move: 100 px/s × 0.5 s = 50 px right.
        assert_eq!(
            right.x.to_bits(),
            (start.x + 50.0).to_bits(),
            "the move scales by speed·dt (100 × 0.5 = 50 px)",
        );

        // Zero stick → unchanged (clamped, but the centre is already inside the window).
        assert_eq!(
            move_cursor(start, Vec2::ZERO, SPEED, 0.5, WINDOW),
            start,
            "a zero stick leaves the cursor where it is",
        );
    }

    /// AC1 — `move_cursor` clamps the result componentwise into `[ZERO, window]`, so the
    /// cursor can never leave the window in any direction.
    #[test]
    fn move_cursor_clamps_inside_the_window() {
        // Push HARD up-left from near the top-left corner: a huge speed·dt would overshoot
        // past (0,0), but the clamp pins it at ZERO.
        let near_corner = Vec2::new(10.0, 10.0);
        let pinned_low = move_cursor(
            near_corner,
            Vec2::new(-1.0, 1.0),
            CursorSpeed::new(10_000.0),
            1.0,
            WINDOW,
        );
        assert_eq!(
            pinned_low,
            Vec2::ZERO,
            "a hard up-left push clamps to the top-left corner (ZERO), never past it",
        );

        // Push HARD down-right from near the far corner: clamps at `window`.
        let near_far = WINDOW - Vec2::new(10.0, 10.0);
        let pinned_high = move_cursor(
            near_far,
            Vec2::new(1.0, -1.0),
            CursorSpeed::new(10_000.0),
            1.0,
            WINDOW,
        );
        assert_eq!(
            pinned_high, WINDOW,
            "a hard down-right push clamps to the window extent, never past it",
        );
    }
}

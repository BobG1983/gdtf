//! The gamepad software cursor state (GTW-259): the [`GamepadCursor`] position, the
//! [`ActivePointer`] arbiter, the [`CursorSpeed`] / [`CursorStickDeadzone`] tunables, and the
//! pure [`move_cursor`] step helper.

use bevy::prelude::*;

/// The gamepad software cursor's SCREEN position, in logical px (window origin TOP-LEFT,
/// y-down — the same frame the OS cursor lives in).
///
/// A NAMED single-field newtype over [`Vec2`] (no-bare-types: the cursor position is a domain
/// value; the inner [`Vec2`] is the framework-math screen-coordinate carve-out the
/// GTW-249/250/251 helpers already use), [`Deref`]ing to its inner [`Vec2`]. A [`Resource`]
/// initialised to a sensible default ([`Self::DEFAULT_CENTRE`]) until the first window-relative
/// move recentres it. [`move_gamepad_cursor`](crate::gamepad::move_gamepad_cursor) writes it; the
/// generalized [`pick_hovered_cell`](crate::pick_hovered_cell) projects it when the gamepad is the
/// active pointer.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursor(pub Vec2);

impl GamepadCursor {
    /// The initial cursor position before the window size is known: a fixed off-origin screen
    /// point so the cursor starts somewhere on a typical window rather than the top-left corner.
    /// The first [`move_gamepad_cursor`](crate::gamepad::move_gamepad_cursor) update clamps it
    /// into the real window extent. A framework-math screen coordinate (the same carve-out the
    /// type uses).
    pub const DEFAULT_CENTRE: Vec2 = Vec2::new(640.0, 360.0);
}

impl Default for GamepadCursor {
    /// The default cursor position ([`Self::DEFAULT_CENTRE`]).
    fn default() -> Self {
        Self(Self::DEFAULT_CENTRE)
    }
}

/// Which input device last moved — the last-moved-wins arbiter the picker reads to choose which
/// cursor to project (GTW-259).
///
/// A domain enum (no-bare-types: the active pointer is a named control-state value, not a bare
/// discriminant), a [`Resource`] defaulting to [`Mouse`](Self::Mouse) (the mouse-only landed
/// behavior until the stick moves). [`move_gamepad_cursor`](crate::gamepad::move_gamepad_cursor)
/// sets [`Gamepad`](Self::Gamepad) when the stick passes the deadzone;
/// [`mouse_reclaims_pointer`](crate::gamepad::mouse_reclaims_pointer) sets [`Mouse`](Self::Mouse)
/// on a [`CursorMoved`](bevy::window::CursorMoved) message.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePointer {
    /// The OS mouse is the active pointer — the picker projects `window.cursor_position()` (the
    /// landed path). The default.
    #[default]
    Mouse,
    /// The gamepad software cursor is the active pointer — the picker projects the
    /// [`GamepadCursor`].
    Gamepad,
}

/// The gamepad cursor's travel speed, in screen px per second.
///
/// A domain quantity (no-bare-types — px/sec is a real unit, never a bare `f32`), a newtype over a
/// private [`f32`] with derived [`Deref`]. An INPUT tunable, so a doc-commented input-level const
/// ([`CURSOR_SPEED`]), NOT a `.ron` data file — FLAGGED for data-driving if the user later wants
/// it authored / hot-swappable (the GTW-250 `PanSpeed` view-const precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorSpeed(f32);

impl CursorSpeed {
    /// Construct a [`CursorSpeed`] from screen px-per-second.
    #[must_use]
    pub const fn new(px_per_second: f32) -> Self {
        Self(px_per_second)
    }
}

/// The left-stick deadzone: a stick magnitude at or below this contributes no cursor move and does
/// NOT claim the pointer for the gamepad.
///
/// A unitless `[0, 1]` analog-stick magnitude threshold (no-bare-types — a newtype over a private
/// [`f32`] with derived [`Deref`]). An INPUT tunable (a doc-commented const
/// [`CURSOR_STICK_DEADZONE`], not `.ron`; FLAGGED for data-driving) so a resting stick never
/// drifts the cursor or steals the pointer from the mouse (the GTW-250 `StickDeadzone` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorStickDeadzone(f32);

impl CursorStickDeadzone {
    /// Construct a [`CursorStickDeadzone`] from a unitless `[0, 1]` magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The shipping gamepad-cursor speed: screen px the software cursor glides per second at full stick
/// (see [`CursorSpeed`]).
///
/// An input const. Chosen so a full-stick sweep crosses a typical window in a second or two;
/// FLAGGED for data-driving later.
pub const CURSOR_SPEED: CursorSpeed = CursorSpeed::new(900.0);

/// The shipping gamepad left-stick deadzone (see [`CursorStickDeadzone`]): a stick magnitude at or
/// below this is ignored so a resting stick never drifts the cursor or claims the pointer.
pub const CURSOR_STICK_DEADZONE: CursorStickDeadzone = CursorStickDeadzone::new(0.15);

/// The new screen position of a software cursor at `pos` after the LEFT stick moves it for `dt`
/// seconds at `speed`, with the stick-y → screen-y flip and a clamp inside the window.
///
/// GTW-259 AC1 — pure, total, unit-tested. The window's origin is TOP-LEFT and screen-y grows
/// DOWNWARD, while a gamepad stick reports UP as `+y`; so a stick pushed UP (`stick.y > 0`) must
/// move the cursor UP on screen (screen-y DECREASES) — the y is FLIPPED:
/// `new = pos + (stick.x, -stick.y) * speed * dt`. The x needs no flip. The result is clamped
/// componentwise into `[Vec2::ZERO, window]` so the cursor can never leave the window. A zero stick
/// leaves `pos` unchanged (clamped, so a pos already inside the window is returned as-is).
///
/// Pure / total — no `App`, no `World`, no side effects. `Vec2` is framework-math plumbing (the
/// GTW-249/250/251 carve-out for raw screen / direction coords), not a domain newtype.
#[must_use]
pub fn move_cursor(pos: Vec2, stick: Vec2, speed: CursorSpeed, dt: f32, window: Vec2) -> Vec2 {
    // Stick-y → screen-y FLIP: stick UP (+y) moves the cursor UP (screen -y).
    let delta = Vec2::new(stick.x, -stick.y) * *speed * dt;
    let moved = pos + delta;
    // Clamp componentwise inside the window so the cursor can never leave it.
    moved.clamp(Vec2::ZERO, window)
}

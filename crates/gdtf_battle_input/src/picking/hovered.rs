//! The hovered cell (GTW-221 / GTW-259): the [`HoveredCell`] resource and the
//! [`pick_hovered_cell`] system that writes it from the ACTIVE pointer's cursor.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{CellLevel, Level};

use crate::{
    gamepad::{ActivePointer, GamepadCursor},
    picking::projection::world_to_cell,
};

/// The cell the OS cursor currently hovers, on the presenter's active level.
///
/// A NAMED newtype over `Option<CellLevel>` (no-bare-types — the hovered cell is
/// a domain value), [`Deref`]ing to its inner [`Option`] so a reader matches it
/// directly. [`None`] means "nothing hovered" — the cursor is off-window, the
/// camera unprojection failed, there is no (single) world camera, or the cursor
/// fell outside the 60×60 grid. The picking system ([`pick_hovered_cell`]) writes
/// it every update; the highlight emitter ([`emit_highlight_request`](crate::emit_highlight_request))
/// reads it and writes the matching `HighlightRequest` for the presenter to draw (GTW-251).
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoveredCell(pub Option<CellLevel>);

/// Writes [`HoveredCell`] from the ACTIVE pointer's cursor every update (GTW-259).
///
/// Reads the single [`WorldCamera`](gdtf_battle_presenter::WorldCamera) (its [`Camera`] +
/// [`GlobalTransform`]), the presenter's [`ActiveLevel`], the last-moved-wins
/// [`ActivePointer`], the primary window (for the OS cursor in
/// [`Mouse`](ActivePointer::Mouse) mode), and the [`GamepadCursor`] (for
/// [`Gamepad`](ActivePointer::Gamepad) mode); CHOOSES the active cursor ([`active_cursor`]);
/// unprojects it with [`Camera::viewport_to_world_2d`]; floors the world point into a cell
/// via [`world_to_cell`]; and writes `HoveredCell(Some(..))` only when the cell is in `0..60`
/// on both axes — otherwise `HoveredCell(None)`.
///
/// The GTW-251 [`emit_highlight_request`](crate::emit_highlight_request) then makes the
/// hover-highlight follow whichever cursor is active.
///
/// Fail-closed (writes `HoveredCell(None)`, NEVER panics) when: there is not EXACTLY one
/// world camera; there is no active cursor (the OS cursor is off-window in `Mouse` mode);
/// `viewport_to_world_2d` returns [`Err`]; or the computed cell is OUTSIDE the grid. The
/// [`Result`] / [`Option`] are handled with `let else` (`bevy-traps.md` #7 — param-only, no
/// `&mut World`). Ordered `.after(move_gamepad_cursor)` (which writes the gamepad cursor +
/// the active pointer) so it reads this update's resolved pointer.
pub fn pick_hovered_cell(
    cameras: Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    active_level: Res<ActiveLevel>,
    active: Res<ActivePointer>,
    gamepad_cursor: Res<GamepadCursor>,
    mut hovered: ResMut<HoveredCell>,
) {
    let resolved =
        resolve_hovered_cell(&cameras, &windows, **active_level, *active, *gamepad_cursor);
    // Write only on a real change so a no-op rewrite does not spuriously trip
    // `Changed<HoveredCell>`.
    if **hovered != resolved {
        *hovered = HoveredCell(resolved);
    }
}

/// The screen cursor the ACTIVE pointer projects: the OS cursor in
/// [`Mouse`](ActivePointer::Mouse) mode (or [`None`] when it is off-window), the
/// [`GamepadCursor`] in [`Gamepad`](ActivePointer::Gamepad) mode (GTW-259).
///
/// The last-moved-wins arbitration: whichever device moved last ([`ActivePointer`]) decides
/// which cursor the picker unprojects. The gamepad cursor is always present (an
/// `init_resource`-d [`Resource`] kept inside the window), so `Gamepad` mode always yields a
/// cursor; the OS cursor is [`None`] off-window.
fn active_cursor(
    window: &Window,
    active: ActivePointer,
    gamepad_cursor: GamepadCursor,
) -> Option<Vec2> {
    match active {
        ActivePointer::Mouse => window.cursor_position(),
        ActivePointer::Gamepad => Some(*gamepad_cursor),
    }
}

/// Resolves the hovered cell from the camera + the ACTIVE pointer's cursor, fail-closed.
///
/// Factored out of [`pick_hovered_cell`] so the resolution logic is a pure
/// `Option`-returning helper (param-only — it borrows the system's queries, never
/// `&mut World`). Reads the [`ActivePointer`] to choose the cursor ([`active_cursor`]): the OS
/// cursor in `Mouse` mode, the [`GamepadCursor`] in `Gamepad` mode (GTW-259). Returns [`None`]
/// for every fail-closed case so the caller simply stores it.
fn resolve_hovered_cell(
    cameras: &Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    level: Level,
    active: ActivePointer,
    gamepad_cursor: GamepadCursor,
) -> Option<CellLevel> {
    // Exactly one world camera, else fail-closed.
    let Ok((camera, cam_transform)) = cameras.single() else {
        return None;
    };
    // Exactly one primary window, else fail-closed.
    let Ok(window) = windows.single() else {
        return None;
    };
    // The active pointer's cursor (OS cursor in Mouse mode, gamepad cursor in Gamepad mode),
    // else fail-closed (the OS cursor is off-window).
    let cursor = active_cursor(window, active, gamepad_cursor)?;
    // The cursor unprojects into the world, else fail-closed.
    let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
        return None;
    };
    world_to_cell(world, level)
}

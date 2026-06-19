//! The inspect target (GTW-221 / GTW-259 / GTW-300): the [`InspectTarget`] resource and the
//! [`pick_hovered_cell`] system that writes its LIVE hovered cell from the ACTIVE pointer's
//! cursor.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{CellLevel, Level};

use crate::{
    gamepad::{ActivePointer, GamepadCursor},
    picking::projection::world_to_cell,
};

/// What the inspect panel should currently describe — the EFFECTIVE inspect target (GTW-300).
///
/// The `(Hovered | Pinned)` MODE the GTW-300 contract names: a NAMED enum (no-bare-types — the
/// inspect mode is a domain decision, not a bare `Option`) so the panel reads ONE value that
/// already encodes the pinned-else-hovered precedence.
///
/// - [`Hovered`](InspectMode::Hovered) — nothing is pinned, so the panel follows the LIVE cursor
///   cell ([`None`] = bare floor / off-map → the panel hides). This is the only variant produced
///   in GTW-300 slice 2 (the pin is never set yet).
/// - [`Pinned`](InspectMode::Pinned) — a target is PINNED to a grid [`CellLevel`]; the panel
///   describes WHATEVER occupies that cell (an enemy occupant OR a cover/wall) regardless of where
///   the cursor now hovers. The pin is a CELL, not an [`Entity`], because cover has NO entity — it
///   is a `(cell, level)`-keyed terrain/ledger value — and a cell uniquely resolves BOTH an enemy
///   (via the grid's occupant lookup) and cover (via terrain/ledger), exactly as the hover path
///   already does. A cell pin also sidesteps the grid's missing entity→cell reverse lookup: the
///   panel resolves the pinned cell directly (slice-3 wiring).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
    /// Follow the LIVE cursor cell — nothing is pinned. [`None`] means "nothing under the cursor".
    Hovered(Option<CellLevel>),
    /// Describe the PINNED grid cell — the panel ignores the cursor while a pin holds (GTW-300).
    Pinned(CellLevel),
}

/// The inspect-panel target: the LIVE hovered cell PLUS an optional PINNED entity (GTW-300).
///
/// A NAMED [`Resource`] (no-bare-types — the inspect target is a domain value) carrying BOTH:
///
/// - the LIVE hovered cell ([`Option<CellLevel>`]) — what the cursor is currently over, written
///   every update by [`pick_hovered_cell`]. The CURSOR-source readers (the reticle emitter
///   [`emit_highlight_request`](crate::emit_highlight_request) and the click/turn decisions in
///   `selection::decision`) read THIS via [`hovered`](InspectTarget::hovered): the reticle follows
///   the live cursor and a click acts on where you clicked.
/// - an optional PINNED grid [`CellLevel`] — set by a click (GTW-300 slice 3). When pinned, the
///   PANEL describes whatever occupies that CELL (an enemy occupant OR a cover/wall) and STOPS
///   tracking the cursor; the live hovered cell keeps updating underneath (independently of the
///   pin) so UN-pinning resumes hover with no stale value. The pin is a CELL — not an [`Entity`] —
///   because cover has no entity (it is a `(cell, level)`-keyed terrain value), so a cell is the
///   one handle that resolves BOTH an enemy and cover the way the hover path already does.
///
/// The PANEL reads [`effective`](InspectTarget::effective), which returns the
/// [`InspectMode`] — `Pinned` when a pin holds, else `Hovered` (the live cell). Keeping the
/// hovered cell INDEPENDENT of the pin is the whole point: the cursor stays available to the
/// reticle / click decisions while the panel is pinned, and unpinning resumes hover instantly.
///
/// Both inner fields are PRIVATE (no-bare-types rule 5) — the picker writes the hovered cell
/// through [`set_hovered`](InspectTarget::set_hovered), the pin through
/// [`set_pinned`](InspectTarget::set_pinned) / [`clear_pin`](InspectTarget::clear_pin), and
/// readers go through [`hovered`](InspectTarget::hovered) / [`pinned`](InspectTarget::pinned) /
/// [`effective`](InspectTarget::effective). [`Default`] is "nothing hovered, nothing pinned".
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectTarget {
    /// The LIVE cell the cursor hovers (the picker writes it). [`None`] = nothing under the cursor.
    hovered: Option<CellLevel>,
    /// The PINNED grid cell, if any — a click sets it; the panel describes it instead of the
    /// cursor. A CELL (not an entity) so it resolves an enemy OR cover alike (GTW-300 slice 3).
    pinned:  Option<CellLevel>,
}

impl InspectTarget {
    /// Constructs an inspect target with the given LIVE hovered cell and NO pin (GTW-300).
    ///
    /// The "nothing pinned" constructor — the only state the picker ever produces in slice 2.
    /// Tests use it to seed the hovered cell directly; the pin is set later via
    /// [`set_pinned`](Self::set_pinned).
    #[must_use]
    pub const fn new(hovered: Option<CellLevel>) -> Self {
        Self {
            hovered,
            pinned: None,
        }
    }

    /// The LIVE hovered cell — what the cursor is currently over (the CURSOR-source read).
    ///
    /// Read by the reticle emitter and the click/turn decisions: those follow the live cursor
    /// REGARDLESS of any pin (a pin only changes what the PANEL shows). [`None`] = nothing hovered.
    #[must_use]
    pub const fn hovered(&self) -> Option<CellLevel> {
        self.hovered
    }

    /// Sets the LIVE hovered cell (the picker's per-update writer). Leaves the pin untouched, so
    /// the cursor keeps tracking under a pin and unpinning resumes hover with a fresh cell.
    pub const fn set_hovered(&mut self, cell: Option<CellLevel>) {
        self.hovered = cell;
    }

    /// The PINNED grid cell, if a pin holds — [`None`] when the panel follows the cursor (GTW-300).
    ///
    /// Set by a click that lands on an enemy or cover (GTW-300 slice 3); the panel resolves the
    /// cell to its occupant / terrain just like the hover path does.
    #[must_use]
    pub const fn pinned(&self) -> Option<CellLevel> {
        self.pinned
    }

    /// PINS the panel to `cell` — the panel describes whatever occupies it (enemy OR cover) until
    /// [`clear_pin`](Self::clear_pin) (GTW-300). The live hovered cell keeps updating underneath,
    /// untouched, so unpinning resumes hover with no stale value. (Slice 3 caller.)
    pub const fn set_pinned(&mut self, cell: CellLevel) {
        self.pinned = Some(cell);
    }

    /// CLEARS the pin so the panel resumes following the live cursor (GTW-300). The live hovered
    /// cell was never paused, so hover resumes with no stale value. (Slice 3 caller.)
    pub const fn clear_pin(&mut self) {
        self.pinned = None;
    }

    /// The EFFECTIVE inspect MODE the panel renders — `Pinned` when a pin holds, else `Hovered`
    /// (the live cell), encoding the contract's pinned-else-hovered precedence in ONE read.
    ///
    /// When a cell is pinned this returns [`InspectMode::Pinned`]`(cell)` so the panel freezes on
    /// that cell regardless of the cursor; with no pin it returns
    /// [`InspectMode::Hovered`]`(self.hovered())`, identical to reading the live hovered cell.
    #[must_use]
    pub const fn effective(&self) -> InspectMode {
        match self.pinned {
            Some(cell) => InspectMode::Pinned(cell),
            None => InspectMode::Hovered(self.hovered),
        }
    }
}

/// Writes [`InspectTarget`]'s LIVE hovered cell from the ACTIVE pointer's cursor every update
/// (GTW-259 / GTW-300).
///
/// Reads the single [`WorldCamera`](gdtf_battle_presenter::WorldCamera) (its [`Camera`] +
/// [`GlobalTransform`]), the presenter's [`ActiveLevel`], the last-moved-wins
/// [`ActivePointer`], the primary window (for the OS cursor in
/// [`Mouse`](ActivePointer::Mouse) mode), and the [`GamepadCursor`] (for
/// [`Gamepad`](ActivePointer::Gamepad) mode); CHOOSES the active cursor ([`active_cursor`]);
/// unprojects it with [`Camera::viewport_to_world_2d`]; floors the world point into a cell
/// via [`world_to_cell`]; and sets the hovered cell to `Some(..)` only when the cell is in `0..60`
/// on both axes — otherwise `None`. It writes ONLY the hovered cell (via
/// [`set_hovered`](InspectTarget::set_hovered)); the pin is never touched here.
///
/// The GTW-251 [`emit_highlight_request`](crate::emit_highlight_request) then makes the
/// hover-highlight follow whichever cursor is active.
///
/// Fail-closed (sets the hovered cell to `None`, NEVER panics) when: there is not EXACTLY one
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
    mut target: ResMut<InspectTarget>,
) {
    let resolved =
        resolve_hovered_cell(&cameras, &windows, **active_level, *active, *gamepad_cursor);
    // Write only on a real change so a no-op rewrite does not spuriously trip
    // `Changed<InspectTarget>`.
    if target.hovered() != resolved {
        target.set_hovered(resolved);
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
///
/// GTW-286 — the cursor is GATED to the map viewport BEFORE unprojection: if it is outside the
/// [`Camera::logical_viewport_rect`] (a margin / a UI panel), or there is no viewport rect at
/// all, the helper returns [`None`] (no hovered cell). Without this gate
/// [`Camera::viewport_to_world_2d`] linearly EXTRAPOLATES a cursor outside the viewport into a
/// valid in-grid world point, so a click over the action-bar margin would floor to an in-grid
/// cell and emit a MOVE/select/fire through the UI; the hover reticle keys off the SAME
/// hovered cell, so the one gate also suppresses the reticle under panels. Mirrors the GTW-271
/// pan-path gate (`pan.rs`), and is fail-closed the same way (no rect → `None`). The cursor and
/// the rect are BOTH logical px (the gamepad software cursor flows through here too, also
/// logical), so the containment test is consistent.
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
    // GTW-286 — gate to the map viewport: a cursor over a margin / UI panel (outside the
    // viewport rect), or no rect at all, yields no hovered cell (fail-closed, matching the
    // GTW-271 pan path). This single chokepoint stops move/select/fire AND the reticle from
    // reaching through the UI, and covers the gamepad software cursor too (both are logical px).
    // `logical_viewport_rect()` is `None` (e.g. headless before `camera_system`) -> fail-closed.
    let viewport = camera.logical_viewport_rect()?;
    if !viewport.contains(cursor) {
        return None;
    }
    // The cursor unprojects into the world, else fail-closed.
    let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
        return None;
    };
    world_to_cell(world, level)
}

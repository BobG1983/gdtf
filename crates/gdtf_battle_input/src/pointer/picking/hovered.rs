//! The inspect target (GTW-221 / GTW-259 / GTW-300): the [`InspectTarget`] resource and the
//! [`pick_hovered_cell`] system that writes its LIVE hovered cell from the ACTIVE pointer's
//! cursor.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::prelude::{CellLevel, Level};

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

/// A read-only view of one UI node's screen geometry — its [`ComputedNode`] (size + border
/// radius), its [`UiGlobalTransform`] (screen-space placement), and its [`InheritedVisibility`]
/// (whether it actually renders) — the EXACT three inputs `bevy_ui`'s own `ui_focus_system`
/// hit-tests the cursor against (GTW-380).
///
/// The cursor-over-UI gate ([`cursor_over_ui`]) iterates this query and tests cursor containment
/// with [`ComputedNode::contains_point`] (the same call `ui_focus_system` makes), skipping any
/// node with a hidden [`InheritedVisibility`]. Grouping the three components into one
/// [`QueryData`](bevy::ecs::query::QueryData) keeps [`pick_hovered_cell`]'s parameter list tidy
/// and names the hit-test's inputs in one place.
#[derive(bevy::ecs::query::QueryData)]
pub struct UiNodeHit {
    /// The node's computed size + border radius — the rect the cursor is tested against.
    node:       &'static ComputedNode,
    /// The node's screen-space placement (physical px) — the rect's position / rotation.
    transform:  &'static UiGlobalTransform,
    /// Whether the node actually renders — a hidden node never absorbs a click.
    visibility: &'static InheritedVisibility,
}

/// Whether the (physical-px) `cursor` falls over ANY visible UI node — the GTW-380
/// click-absorption gate.
///
/// Iterates every UI node ([`UiNodeHit`]) and returns `true` as soon as one VISIBLE node
/// contains the cursor, using the SAME [`ComputedNode::contains_point`] test `bevy_ui`'s
/// `ui_focus_system` uses (so a click the HUD would consume agrees with this gate). Hidden nodes
/// (a `false` [`InheritedVisibility`]) are skipped — they draw nothing, so they absorb nothing.
///
/// Panel-agnostic by construction: it queries the BUILT-IN [`ComputedNode`] every UI node carries
/// rather than any app-crate panel marker (the dep edge is `input -> presenter -> sim`, so the
/// input crate cannot — and must not — name `gdtf_app`'s panel roots). So it catches the bare
/// `Node` panels (the status / inspect panels, which carry NO [`Interaction`], so a
/// `Query<&Interaction>` would MISS them) just as it catches the action-bar buttons.
///
/// `cursor` is PHYSICAL px to match [`ComputedNode`] / [`UiGlobalTransform`] (both physical),
/// exactly as `ui_focus_system` feeds it `window.physical_cursor_position()`.
fn cursor_over_ui(ui_nodes: &Query<UiNodeHit>, cursor: Vec2) -> bool {
    ui_nodes
        .iter()
        .any(|hit| hit.visibility.get() && hit.node.contains_point(*hit.transform, cursor))
}

/// Writes [`InspectTarget`]'s LIVE hovered cell from the ACTIVE pointer's cursor every update
/// (GTW-259 / GTW-300), UNLESS the pointer is over the HUD (GTW-380).
///
/// Reads the single [`WorldCamera`](gdtf_battle_presenter::WorldCamera) (its [`Camera`] +
/// [`GlobalTransform`]), the presenter's [`ActiveLevel`], the last-moved-wins
/// [`ActivePointer`], the primary window (for the OS cursor in
/// [`Mouse`](ActivePointer::Mouse) mode), the [`GamepadCursor`] (for
/// [`Gamepad`](ActivePointer::Gamepad) mode), and EVERY UI node's [`ComputedNode`] geometry (the
/// GTW-380 over-UI gate); CHOOSES the active cursor (`active_cursor`); unprojects it with
/// [`Camera::viewport_to_world_2d`]; floors the world point into a cell via [`world_to_cell`]; and
/// sets the hovered cell to `Some(..)` only when the cell is in `0..60` on both axes — otherwise
/// `None`. It writes ONLY the hovered cell (via [`set_hovered`](InspectTarget::set_hovered)); the
/// pin is never touched here.
///
/// GTW-380 — the UI ABSORBS clicks over its own nodes: when the active cursor is over ANY visible
/// UI node (`cursor_over_ui`), the hovered cell resolves to `None`, so the board click decision
/// (which keys off [`InspectTarget::hovered`]) and the hover reticle (which keys off the same cell)
/// both see "nothing under the cursor" — the click NEVER reaches the board cell-picker. The bare
/// `Node` HUD panels carry no [`Interaction`], so this hit-tests the built-in [`ComputedNode`]
/// geometry rather than `Interaction`; it complements (does not replace) the GTW-286 map-viewport
/// gate, which only excludes the area OUTSIDE the world viewport (the panels are absolute overlays
/// drawn INSIDE it, so they slip through the viewport gate — this is the bug GTW-380 fixes).
///
/// The GTW-251 [`emit_highlight_request`](crate::emit_highlight_request) then makes the
/// hover-highlight follow whichever cursor is active.
///
/// Fail-closed (sets the hovered cell to `None`, NEVER panics) when: there is not EXACTLY one
/// world camera; there is no active cursor (the OS cursor is off-window in `Mouse` mode); the
/// cursor is over the HUD (GTW-380); `viewport_to_world_2d` returns [`Err`]; or the computed cell
/// is OUTSIDE the grid. The [`Result`] / [`Option`] are handled with `let else` (`bevy-traps.md`
/// #7 — param-only, no `&mut World`). Ordered `.after(move_gamepad_cursor)` (which writes the
/// gamepad cursor + the active pointer) so it reads this update's resolved pointer; `ui_focus_system`
/// already wrote each node's `ComputedNode` / `UiGlobalTransform` in `PreUpdate`, so the over-UI
/// gate reads this frame's settled geometry.
pub fn pick_hovered_cell(
    cameras: Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_nodes: Query<UiNodeHit>,
    active_level: Res<ActiveLevel>,
    active: Res<ActivePointer>,
    gamepad_cursor: Res<GamepadCursor>,
    mut target: ResMut<InspectTarget>,
) {
    let resolved = resolve_hovered_cell(
        &cameras,
        &windows,
        &ui_nodes,
        **active_level,
        *active,
        *gamepad_cursor,
    );
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
/// [`Camera::logical_viewport_rect`] (a margin), or there is no viewport rect at
/// all, the helper returns [`None`] (no hovered cell). Without this gate
/// [`Camera::viewport_to_world_2d`] linearly EXTRAPOLATES a cursor outside the viewport into a
/// valid in-grid world point, so a click over the bottom-bar margin would floor to an in-grid
/// cell and emit a MOVE/select/fire through the UI; the hover reticle keys off the SAME
/// hovered cell, so the one gate also suppresses the reticle under that margin. Mirrors the
/// GTW-271 pan-path gate (`pan.rs`), and is fail-closed the same way (no rect → `None`). The
/// cursor and the rect are BOTH logical px (the gamepad software cursor flows through here too,
/// also logical), so the containment test is consistent.
///
/// GTW-380 — the cursor is ALSO gated on pointer-over-UI ([`cursor_over_ui`]): the GTW-286
/// viewport gate only excludes the area OUTSIDE the world viewport, but the HUD panels are
/// absolute OVERLAYS drawn INSIDE it (only the bottom bar reduces the viewport), so a click on a
/// panel passes the viewport gate and would fall through to the board. This gate hit-tests the
/// active cursor against every visible UI node's [`ComputedNode`] geometry (PHYSICAL px, so the
/// logical cursor is scaled up by the window `scale_factor`) and returns [`None`] when the cursor
/// is over ANY panel — so the UI absorbs the click and the board never sees it.
fn resolve_hovered_cell(
    cameras: &Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_nodes: &Query<UiNodeHit>,
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
    // else fail-closed (the OS cursor is off-window). LOGICAL px.
    let cursor = active_cursor(window, active, gamepad_cursor)?;
    // GTW-380 — UI ABSORBS clicks over its own nodes: if the cursor is over any visible HUD
    // panel/button, resolve to no hovered cell so the board click + reticle never reach through
    // it. The over-UI test uses `ComputedNode` geometry, which is PHYSICAL px, so the logical
    // cursor is scaled to physical by the window `scale_factor` to match `ui_focus_system`.
    if cursor_over_ui(ui_nodes, cursor * window.scale_factor()) {
        return None;
    }
    // GTW-286 — gate to the map viewport: a cursor over a margin (outside the viewport rect),
    // or no rect at all, yields no hovered cell (fail-closed, matching the GTW-271 pan path).
    // This single chokepoint stops move/select/fire AND the reticle from reaching through the
    // bottom-bar margin, and covers the gamepad software cursor too (both are logical px).
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

//! Battle input layer for GDTF — the HEAD of the `input -> presenter -> sim`
//! chain.
//!
//! This crate gives the landed top-down SPRITE battle (the read-only presenter,
//! S2-S6) its first cursor awareness. Every update during a live battle it:
//!
//! 1. reads the presenter's [`WorldCamera`] and the primary window's OS cursor,
//! 2. unprojects the cursor into the world via the camera's
//!    [`Camera::viewport_to_world_2d`],
//! 3. floors that world point into a sim [`Cell`] on the presenter's
//!    [`ActiveLevel`] — the INVERSE of the presenter's forward
//!    [`cell_to_world`] projection — and stores the result in the
//!    [`HoveredCell`] resource, and
//! 4. snaps ONE hover-highlight sprite onto that cell (hidden when nothing is
//!    hovered).
//!
//! # The one-way dependency chain (ADR-0001)
//!
//! The dependency edge runs strictly
//! `gdtf_battle_input -> gdtf_battle_presenter -> gdtf_battle_sim` — a CHAIN,
//! never a cycle. This crate reads the presenter's camera/px/level interface
//! ([`WorldCamera`] + [`WORLD_RENDER_LAYER`], [`CELL_PX`] + [`cell_to_world`],
//! [`ActiveLevel`]) and the sim's presentation-agnostic metric ([`Cell`] /
//! [`Level`] / [`CellLevel`]); the presenter reads only the sim; the sim reads
//! NEITHER. The top-down sprite presenter is the swappable VIEW an iso renderer
//! (GTW-49 / GTW-10) later replaces WITHOUT touching this input crate, because
//! input speaks cursor + [`Cell`], not pixels — the px boundary
//! ([`CELL_PX`] / [`cell_to_world`]) lives in the presenter, and the world->cell
//! inverse added here reuses it (it never recomputes the px scale).
//!
//! # Scope (GTW-221 / GTW-48 S7)
//!
//! This slice touches NOTHING in the sim's authoritative state: it emits NO act
//! message (`FireRequested` / `Set*Requested` are S8 / GTW-222), re-registers NO
//! sim plugin or lifecycle message (E10 owns all setup/teardown), and adds NO
//! selection / targeting / HUD logic. It only READS sim types + the presenter's
//! interface and WRITES the [`HoveredCell`] resource + one highlight sprite.

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, WorldCamera, cell_to_world};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, GRID_HEIGHT, GRID_WIDTH, Level};

/// The cell the OS cursor currently hovers, on the presenter's active level.
///
/// A NAMED newtype over `Option<CellLevel>` (no-bare-types — the hovered cell is
/// a domain value), [`Deref`]ing to its inner [`Option`] so a reader matches it
/// directly. [`None`] means "nothing hovered" — the cursor is off-window, the
/// camera unprojection failed, there is no (single) world camera, or the cursor
/// fell outside the 60×60 grid. The picking system ([`pick_hovered_cell`]) writes
/// it every update; the highlight system ([`update_hover_highlight`]) reads it.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoveredCell(pub Option<CellLevel>);

/// Marker resource the [`GdtfBattleInputPlugin`] inserts on `build`.
///
/// Its presence in the world is the test-observable proof that the input plugin's
/// `build` actually ran inside the real scene stack (AC1 asserts it present after
/// the harness descends to the battle). A framework type (`Resource`), so it is
/// exempt from the no-bare-types rule.
#[derive(Resource)]
pub struct GdtfBattleInputActive;

/// Marker for the single hover-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the
/// same justification the presenter's `WorldCamera` / `TerrainSprite` markers use):
/// the highlight system queries `With<HoverHighlight>` to find and MOVE the one
/// existing highlight rather than spawning a duplicate each update.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct HoverHighlight;

/// The translucent tint of the hover-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a
/// domain quantity (the `CELL_PX`-class const carve-out). A solid-color reticle is
/// the engineer's-choice highlight visual this slice sanctions: there is no
/// `SheetRole::Ui` / reticle role in the landed atlas (only Terrain / Characters /
/// Effects), so a tinted solid `Sprite` is the renderer-agnostic option. A faint
/// warm-white at low alpha so it reads as a highlight OVER the cell without hiding
/// the tile beneath.
const HIGHLIGHT_TINT: Color = Color::srgba(1.0, 0.95, 0.6, 0.35);

/// The input plugin: cursor->cell picking + the hover-highlight sprite.
///
/// Added by `gdtf_app`'s `GameBattleScapeScenePlugin` BESIDE
/// `BattlePresenterPlugin::default()`, so its `build` runs when the scene plugins
/// register. On `build` it inserts the [`GdtfBattleInputActive`] marker, initialises
/// the [`HoveredCell`] resource (so a reader never hits a missing resource), and
/// registers [`pick_hovered_cell`] + [`update_hover_highlight`] in [`Update`],
/// both gated `run_if(resource_exists::<BattleInProgress>)` (the sim's live-battle
/// witness — the same gate the S4-S6 draw systems use) so the cursor is inert
/// pre-battle.
///
/// Ordering vs the sim's `SimSystems::Simulate` band is NOT load-bearing — picking
/// reads only the camera + window (no sim-component state mutated that frame) — so
/// no `.after` is required; the battle gate IS. The highlight runs
/// `.after(pick_hovered_cell)` so it observes the SAME update's [`HoveredCell`].
pub struct GdtfBattleInputPlugin;

impl Plugin for GdtfBattleInputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GdtfBattleInputActive)
            .init_resource::<HoveredCell>()
            .add_systems(
                Update,
                (
                    pick_hovered_cell,
                    update_hover_highlight.after(pick_hovered_cell),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}

/// Writes [`HoveredCell`] from the OS cursor every update.
///
/// Reads the single [`WorldCamera`] (its [`Camera`] + [`GlobalTransform`]), the
/// primary window's cursor ([`Window::cursor_position`]), and the presenter's
/// [`ActiveLevel`]; unprojects the cursor with
/// [`Camera::viewport_to_world_2d`]; floors the world point into a [`Cell`] via
/// [`world_to_cell`]; and writes `HoveredCell(Some(..))` only when the cell is in
/// `0..60` on both axes — otherwise `HoveredCell(None)`.
///
/// Fail-closed (writes `HoveredCell(None)`, NEVER panics) when: there is not
/// EXACTLY one world camera; there is no cursor (it is off-window);
/// `viewport_to_world_2d` returns [`Err`]; or the computed cell is OUTSIDE the
/// grid. The [`Result`] / [`Option`] are handled explicitly with `let else`
/// (`bevy-traps.md` #7 — param-only, no `&mut World`).
pub fn pick_hovered_cell(
    cameras: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    active_level: Res<ActiveLevel>,
    mut hovered: ResMut<HoveredCell>,
) {
    let resolved = resolve_hovered_cell(&cameras, &windows, **active_level);
    // Write every update; change-detection avoids a no-op rewrite spuriously
    // tripping `Changed<HoveredCell>` only when the value actually differs.
    if **hovered != resolved {
        *hovered = HoveredCell(resolved);
    }
}

/// Resolves the hovered cell from the camera + window queries, fail-closed.
///
/// Factored out of [`pick_hovered_cell`] so the resolution logic is a pure
/// `Option`-returning helper (param-only — it borrows the system's queries, never
/// `&mut World`). Returns [`None`] for every fail-closed case so the caller simply
/// stores it.
fn resolve_hovered_cell(
    cameras: &Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    level: Level,
) -> Option<CellLevel> {
    // Exactly one world camera, else fail-closed.
    let Ok((camera, cam_transform)) = cameras.single() else {
        return None;
    };
    // Exactly one primary window, else fail-closed.
    let Ok(window) = windows.single() else {
        return None;
    };
    // The cursor is on-window, else fail-closed.
    let cursor = window.cursor_position()?;
    // The cursor unprojects into the world, else fail-closed.
    let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
        return None;
    };
    world_to_cell(world, level)
}

/// The INVERSE of the presenter's [`cell_to_world`]: a world point -> the
/// `(cell, level)` it falls in, or [`None`] when the cell is outside the 60×60
/// grid.
///
/// The forward map is `cell_to_world(cell, level) = (cell.x * CELL_PX, -(cell.y) *
/// CELL_PX, z_for(level))`, so the inverse floors the scaled world coordinates
/// (reusing [`CELL_PX`] — the presenter's SINGLE px source of truth, never a
/// recomputed `16.0`):
///
/// - `cell.x = floor(world.x / CELL_PX)`
/// - `cell.y = floor(-world.y / CELL_PX)` — note the sign: the forward map negates
///   `cell.y`, so the inverse negates `world.y`.
///
/// Floored, never rounded — the metric's floor-not-round contract (mirroring
/// `pos_to_cell` in the sim). Returns [`None`] (nothing hovered) when the cell is
/// outside `0..`[`GRID_WIDTH`] × `0..`[`GRID_HEIGHT`] (= 60×60), so the caller
/// writes `HoveredCell(None)` for an off-grid cursor.
#[must_use]
pub fn world_to_cell(world: Vec2, level: Level) -> Option<CellLevel> {
    let cx = floor_to_cell_coord(world.x / CELL_PX);
    let cy = floor_to_cell_coord(-world.y / CELL_PX);
    if in_grid(cx, cy) {
        Some(CellLevel::new(Cell::new(cx, cy), level))
    } else {
        None
    }
}

/// Whether `(cx, cy)` is inside the 60×60 grid (`0..`[`GRID_WIDTH`] ×
/// `0..`[`GRID_HEIGHT`]).
///
/// The bounds are the sim's structural [`GRID_WIDTH`] / [`GRID_HEIGHT`] (= 60),
/// widened to `i32` so a negative (above/left of origin) coordinate is rejected
/// rather than wrapping. Reuses [`grid_extent_i32`] for the `usize -> i32` widen so
/// the cast never wraps in an unguarded `as`.
fn in_grid(cx: i32, cy: i32) -> bool {
    (0..grid_extent_i32(GRID_WIDTH)).contains(&cx)
        && (0..grid_extent_i32(GRID_HEIGHT)).contains(&cy)
}

/// Widens a `usize` grid extent (`GRID_WIDTH` / `GRID_HEIGHT`) to `i32` for a cell
/// bounds check.
///
/// 60 fits `i32` comfortably, but `as i32` on a `usize` trips `cast_possible_wrap`
/// (`-D`); [`i32::try_from`] is the no-`unwrap` cast — an unrepresentable extent
/// saturates to [`i32::MAX`], which only makes the bounds check MORE permissive on
/// a (impossible) absurd grid, never narrower.
fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

/// Floors a scaled world coordinate to its integer cell index, clamped into the
/// `i32` range so a wild (or `NaN`) coordinate can never wrap on the cast.
///
/// Mirrors the sim metric's `floor_to_i32`: [`f32::floor`] so a negative coordinate
/// goes to the lower integer (−0.5 → −1, the floor-not-round contract), then a
/// clamp into the `i32` range before the cast. A `NaN` clamps to `0` (the
/// out-of-grid path then rejects it via [`in_grid`]).
const fn floor_to_cell_coord(scaled: f32) -> i32 {
    let floored = scaled.floor();
    // `clamp` orders `MIN`/`MAX` so a `NaN` resolves to the lower bound, then is
    // rejected by the grid bounds check.
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped into the i32 range above, so the cast cannot wrap; the fractional part is gone after floor"
    )]
    let coord = clamped as i32;
    coord
}

/// Maintains exactly ONE hover-highlight sprite that snaps to [`HoveredCell`].
///
/// Spawns the single [`HoverHighlight`] sprite the first time it is needed; on every
/// later update it MOVES that one sprite's [`Transform`] to
/// [`cell_to_world`]`(hovered cell, hovered level)` and shows it when [`HoveredCell`]
/// is [`Some`], or HIDES it ([`Visibility::Hidden`]) when [`None`] — so no duplicate
/// highlight sprites accumulate. The sprite is sized to exactly one cell
/// (`custom_size: Some(Vec2::splat(CELL_PX))`, the S3/S4 sizing recipe) and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the
/// battlefield, not the GTW-120 UI camera.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn, a
/// `Query<(&mut Transform, &mut Visibility), With<HoverHighlight>>` for the move +
/// show/hide. Runs `.after(pick_hovered_cell)` so it reads the same update's
/// [`HoveredCell`].
pub fn update_hover_highlight(
    mut commands: Commands,
    hovered: Res<HoveredCell>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<HoverHighlight>>,
) {
    // The world position + visibility the one highlight should take this update.
    let target = (**hovered)
        .map(|cell| cell_to_world(Cell::new(cell.x, cell.y), Level::new(level_index(cell))));

    match highlights.single_mut() {
        Ok((mut transform, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        // No highlight yet: spawn the single sprite the first time a cell is hovered.
        // (When nothing is hovered there is nothing to spawn — it stays absent until
        // the first hover, which is equivalent to "hidden".)
        Err(_) => {
            if let Some(world) = target {
                commands.spawn((
                    HoverHighlight,
                    Sprite {
                        color: HIGHLIGHT_TINT,
                        custom_size: Some(Vec2::splat(CELL_PX)),
                        ..default()
                    },
                    Transform::from_translation(world),
                    // Explicitly Visible (not the `Inherited` default) so the reticle
                    // shows from the first frame it is hovered, independent of any
                    // parent visibility.
                    Visibility::Visible,
                    RenderLayers::layer(WORLD_RENDER_LAYER),
                ));
            }
        }
    }
}

/// The storey index of a [`CellLevel`]'s `z`, narrowed to the [`Level`]'s `u8`.
///
/// A [`CellLevel`] `Deref`s to an `IVec3` whose `z` is a storey index built from a
/// [`Level`] (always `0..`[`gdtf_battle_sim::MAX_LEVELS`], well within `u8`). The
/// clamped [`u8::try_from`] is the no-`unwrap` narrow — a (impossible) out-of-range
/// `z` saturates to [`u8::MAX`] rather than panicking.
fn level_index(cell: CellLevel) -> u8 {
    u8::try_from(cell.z).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC2 — the world->cell inverse is the documented floored inverse of
    /// `cell_to_world` (a coordinate-system FACT, pinned the way the forward map's
    /// `cell_to_world_projects_row_zero_to_the_top` is pinned).
    ///
    /// For a handful of representative cells, `cell_to_world(cell, L)` then
    /// `world_to_cell(that world (x,y), L)` must round-trip back to the SAME cell.
    #[test]
    fn world_to_cell_inverts_cell_to_world() {
        let level = Level::new(0);
        for (cx, cy) in [(0, 0), (1, 2), (12, 7), (59, 59), (30, 0)] {
            let cell = Cell::new(cx, cy);
            let world = cell_to_world(cell, level);
            let back = world_to_cell(world.truncate(), level);
            assert_eq!(
                back,
                Some(CellLevel::new(cell, level)),
                "cell ({cx},{cy}) must round-trip through cell_to_world -> world_to_cell",
            );
        }
    }

    /// AC2 — a world point INSIDE a cell (not on the corner) still floors into that
    /// cell: a point 0.5 cell past the corner maps to the corner's cell, proving
    /// the inverse FLOORS (never rounds).
    #[test]
    fn world_to_cell_floors_within_a_cell() {
        let level = Level::new(0);
        // The interior of cell (3, 4): half a cell past its corner on each axis.
        // x = (3 + 0.5) * CELL_PX ; y = -((4 + 0.5) * CELL_PX) (the negated row).
        let interior = Vec2::new(3.5 * CELL_PX, -4.5 * CELL_PX);
        assert_eq!(
            world_to_cell(interior, level),
            Some(CellLevel::new(Cell::new(3, 4), level)),
            "an interior world point must floor into its containing cell",
        );
    }

    /// AC3 — the inverse fails closed to `None` for an off-grid world point: a
    /// negative-x cursor (left of column 0) and a beyond-row-59 cursor both yield
    /// `None`.
    #[test]
    fn world_to_cell_off_grid_is_none() {
        let level = Level::new(0);
        // Left of column 0 (cell.x = floor(-1) = -1, outside 0..60).
        assert_eq!(
            world_to_cell(Vec2::new(-CELL_PX, 0.0), level),
            None,
            "a world point left of column 0 must be None",
        );
        // Below row 59: world.y = -(60 * CELL_PX) => cell.y = 60, outside 0..60.
        assert_eq!(
            world_to_cell(Vec2::new(0.0, -60.0 * CELL_PX), level),
            None,
            "a world point below row 59 must be None",
        );
    }

    /// `level_index` narrows a `CellLevel`'s storey z back to the `u8` the `Level`
    /// carries, so the highlight redraws on the hovered level.
    #[test]
    fn level_index_recovers_the_storey() {
        let key = CellLevel::new(Cell::new(2, 2), Level::new(3));
        assert_eq!(level_index(key), 3, "level_index must recover the storey z");
    }
}

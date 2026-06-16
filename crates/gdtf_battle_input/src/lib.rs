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
//! 4. EMITS a presenter-owned [`HighlightRequest`] reflecting that hovered cell
//!    (the presenter LISTENS and draws the reticle — GTW-251).
//!
//! # The hover-highlight is message-driven (GTW-251)
//!
//! As of GTW-251 this crate no longer DRAWS the hover-highlight. The sprite +
//! drawing moved to the presenter (`gdtf_battle_presenter::highlight`); input now
//! EMITS the presenter-defined [`HighlightRequest`] message after the picker resolves
//! [`HoveredCell`], and the presenter's `draw_highlight_on_request` system moves/shows/
//! hides the one reticle sprite from it. The behavior is identical (the highlight
//! still follows the hovered cell) — this is a refactor onto the message-driven I/O
//! boundary, and the seam the gamepad cursor (GTW-259) builds on. The presenter, as
//! the CONSUMER, defines the request type, so the crate edge stays one-way
//! (`input → presenter`, never a cycle): input names a presenter-defined message; the
//! presenter never names input.
//!
//! # The one-way dependency chain (ADR-0001)
//!
//! The dependency edge runs strictly
//! `gdtf_battle_input -> gdtf_battle_presenter -> gdtf_battle_sim` — a CHAIN,
//! never a cycle. This crate reads the presenter's camera/px/level interface
//! ([`WorldCamera`] + [`WORLD_RENDER_LAYER`](gdtf_battle_presenter::WORLD_RENDER_LAYER),
//! [`CELL_PX`] + [`cell_to_world`], [`ActiveLevel`]) and the sim's presentation-agnostic metric ([`Cell`] /
//! [`Level`] / [`CellLevel`]); the presenter reads only the sim; the sim reads
//! NEITHER. The top-down sprite presenter is the swappable VIEW an iso renderer
//! (GTW-49 / GTW-10) later replaces WITHOUT touching this input crate, because
//! input speaks cursor + [`Cell`], not pixels — the px boundary
//! ([`CELL_PX`] / [`cell_to_world`]) lives in the presenter, and the world->cell
//! inverse added here reuses it (it never recomputes the px scale).
//!
//! # Scope (GTW-221 / GTW-48 S7 + GTW-225 / S8 selection substrate)
//!
//! GTW-221 (S7) added cursor->cell picking + the hover-highlight sprite, touching
//! NOTHING in the sim's authoritative state.
//!
//! GTW-225 (S8) adds the SELECTION substrate the act surfaces ride: the
//! [`SelectedShooter`](selection::SelectedShooter) resource set by left-clicking an
//! occupied cell (faction-agnostic — own-ganger-only is deferred to GTW-226), the
//! [`SelectionHighlight`](selection::SelectionHighlight) sprite, level up/down
//! cycling of the presenter's [`ActiveLevel`], the FIRST data-driven keybind
//! [`Keybinds`](keybinds::Keybinds) table, and the shared ACT-INTENT data seam
//! ([`PendingActIntent`](intent::PendingActIntent) + the ONE
//! [`dispatch_act_intents`](intent::dispatch_act_intents) drain) that BOTH the 222b
//! keyboard systems AND the 222c `gdtf_app` buttons write. This slice still emits NO
//! sim act message (`FireRequested` / `Set*Requested` are 222b) and re-registers NO
//! sim plugin or lifecycle message (E10 owns all setup/teardown): it only READS sim
//! types + the presenter's interface and WRITES input-layer state (the hovered cell,
//! the selection, the active level, the intent queue) + its two highlight sprites.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, HighlightRequest, WorldCamera};
use gdtf_battle_sim::{
    BattleInProgress, Cell, CellLevel, GRID_HEIGHT, GRID_WIDTH, Level, OccupancyGrid,
    PlayerFaction,
    acts::{
        FireRequested, MoveRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
    },
    occupancy_sync::SimSystems,
    tuning::CombatTuning,
};

pub mod cycle;
pub mod fire_mode;
pub mod fire_surface;
pub mod intent;
pub mod keybinds;
pub mod keyboard;
pub mod selection;

pub use cycle::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
pub use fire_mode::{SelectedFireMode, next_fire_mode, sync_fire_mode_on_select};
pub use intent::{
    ActIntent, ActWriters, LevelStep, PendingActIntent, dispatch_act_intents, step_level,
};
pub use keybinds::{BoundKey, Keybinds, KeybindsHandle, load_keybinds, resolve_keybinds};
pub use keyboard::{fire_mode_cycle_key, level_keys, posture_keys, select_clear_key};
pub use selection::{
    LeftClickReads, SelectedShooter, SelectionHighlight, auto_select_first_player_ganger,
    left_click_act, right_click_turn_to_face, update_selection_highlight,
};

/// The cell the OS cursor currently hovers, on the presenter's active level.
///
/// A NAMED newtype over `Option<CellLevel>` (no-bare-types — the hovered cell is
/// a domain value), [`Deref`]ing to its inner [`Option`] so a reader matches it
/// directly. [`None`] means "nothing hovered" — the cursor is off-window, the
/// camera unprojection failed, there is no (single) world camera, or the cursor
/// fell outside the 60×60 grid. The picking system ([`pick_hovered_cell`]) writes
/// it every update; the highlight emitter ([`emit_highlight_request`]) reads it and
/// writes the matching [`HighlightRequest`] for the presenter to draw (GTW-251).
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

/// Input system-ordering anchor — the named `Update`-schedule band every battle input
/// system runs in, configured `.before(`[`SimSystems::Simulate`]`)`.
///
/// Mirrors the sim's [`SimSystems::Simulate`] and the presenter's `PresenterSystems::Draw`
/// anchors (the proven cross-crate ordering pattern): the band is defined ONCE via
/// [`configure_sets`](App::configure_sets), then `.in_set` on each member system
/// (`bevy-traps.md` #5 — `configure_sets` precedes `.in_set`). Putting the whole input
/// band `.before(SimSystems::Simulate)` realizes the documented one-way
/// `input -> sim -> presenter` loop (ADR-0001) inside a single `Update`: a click's
/// `*Requested` message is EMITTED before the sim consumes it the SAME frame, removing the
/// input/sim ordering ambiguity (`bevy-traps.md` #3 — no flaky one-frame lag). The set
/// membership is purely additive — every intra-band `.before`/`.after` edge and `run_if`
/// gate is unchanged, and those inner edges all live WITHIN this set, so they compose with
/// the cross-band edge. The ordering edge is independent of the sim band's `run_if` gate
/// (an edge is not a run condition), so it is correct whether or not a battle is live.
///
/// A framework `SystemSet` label, not a domain value (the no-bare-types framework carve-out,
/// the same justification [`SimSystems`] / `PresenterSystems` use) — `pub` so the app and
/// tests can name it for ordering and probing.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSystems {
    /// The band holding every battle input system — ordered before the sim's world
    /// mutations so a `*Requested` message is emitted the same update the sim runs.
    Gather,
}

/// The input plugin: cursor->cell picking + hover-highlight (S7), ganger selection,
/// level cycling, the data-driven keybinds, and the shared act-intent seam (S8).
///
/// Added by `gdtf_app`'s `GameBattleScapeScenePlugin` BESIDE
/// `BattlePresenterPlugin::default()`, so its `build` runs when the scene plugins
/// register. On `build` it:
///
/// - inserts the [`GdtfBattleInputActive`] marker and initialises the [`HoveredCell`]
///   (S7), [`SelectedShooter`], [`SelectedFireMode`] (222b), and [`PendingActIntent`]
///   resources (so a reader never hits a missing resource), and registers the five
///   `*Requested` message buffers the drain emits (the four landed acts + GTW-238's
///   [`MoveRequested`](gdtf_battle_sim::acts::MoveRequested));
/// - registers the S7 cursor picker ([`pick_hovered_cell`]) + the GTW-251 highlight
///   EMITTER ([`emit_highlight_request`]) that writes a presenter-owned
///   [`HighlightRequest`] reflecting the picked [`HoveredCell`] (the presenter draws
///   it), and registers the [`HighlightRequest`] buffer so the emitter's
///   [`MessageWriter`] validates headlessly (`bevy-traps.md` #4);
/// - registers the GTW-238 unified left-click decision ([`left_click_act`]) + the
///   right-click turn-to-face surface ([`right_click_turn_to_face`]) — replacing the
///   GTW-225/227 `select_on_click` + `fire_on_click` race — the GTW-255 battle-start
///   auto-select ([`auto_select_first_player_ganger`], `.before(left_click_act)`, which
///   sets the INITIAL [`SelectedShooter`] once when the battle opens with nothing
///   selected), plus the selection
///   highlight ([`update_selection_highlight`]), the 222b fire-mode default-on-select
///   ([`sync_fire_mode_on_select`]), the keyboard press surface (S8 [`level_keys`] /
///   [`select_clear_key`] + 222b [`posture_keys`] / [`fire_mode_cycle_key`]), and the
///   ONE intent drain ([`dispatch_act_intents`]) that emits the act `*Requested` for
///   every queued act-bearing intent; and
/// - loads the data-driven keybind table ([`load_keybinds`] / [`resolve_keybinds`])
///   via the GTW-136 [`RonAsset<T>`](gdtf_assets::RonAsset) path, gated on an
///   [`AssetServer`] so a `MinimalPlugins` headless app no-ops (the presenter's
///   `tile_roles` load precedent, `bevy-traps.md` #1).
///
/// All live-battle systems are gated `run_if(resource_exists::<BattleInProgress>)`
/// (the sim's live-battle witness — the same gate the S4-S6 draw systems use) so the
/// input layer is inert pre-battle (AC11). The GTW-238 click decision additionally
/// gates on [`PlayerFaction`] (the battle-scoped friend/foe witness) + the occupancy /
/// mouse / tuning it reads.
///
/// Ordering (`bevy-traps.md` #3): the highlight emitter runs `.after(pick_hovered_cell)`
/// and the selection highlight `.after(left_click_act)` so each observes the SAME
/// update's resolved cell/selection. [`left_click_act`] / [`right_click_turn_to_face`]
/// run `.before(pick_hovered_cell)` (the cell resolved last update) and
/// `.before(dispatch_act_intents)` (the drain). The intent drain
/// ([`dispatch_act_intents`]) runs `.after` EVERY intent WRITER (the keyboard press
/// systems + the click decision) so it drains the same update's pushes — and the 222c
/// `gdtf_app` buttons that push the same seam run in `Update` upstream of it.
pub struct GdtfBattleInputPlugin;

impl Plugin for GdtfBattleInputPlugin {
    fn build(&self, app: &mut App) {
        // GTW-245 — anchor the whole input band BEFORE the sim band, realizing the
        // one-way `input -> sim -> presenter` loop (ADR-0001) inside one `Update`.
        // Defined ONCE here, before `add_systems` (`bevy-traps.md` #5 — `configure_sets`
        // precedes `.in_set`); each input system below carries `.in_set(InputSystems::Gather)`.
        // `configure_sets` ACCUMULATES across plugins (`bevy-traps.md` #5), so this edge
        // composes with `OccupancyMaintenancePlugin`'s `configure_sets(Update,
        // SimSystems::Simulate)` and the presenter's `PresenterSystems::Draw.after(...)` —
        // resolving the three sets to input -> sim -> presenter. The edge is independent
        // of the sim band's `run_if` gate (an edge is not a run condition), so it holds
        // whether or not a battle is live. `SimSystems` is path-qualified from
        // `gdtf_battle_sim` (the anchor's sole owner) exactly as the presenter references it.
        app.configure_sets(Update, InputSystems::Gather.before(SimSystems::Simulate))
            .insert_resource(GdtfBattleInputActive)
            .init_resource::<HoveredCell>()
            .init_resource::<SelectedShooter>()
            .init_resource::<SelectedFireMode>()
            .init_resource::<PendingActIntent>()
            // The seam EMITS these `*Requested` messages — register the five buffers
            // the ONE `dispatch_act_intents` drain writes into (the four landed acts +
            // GTW-238's `MoveRequested`) so its `MessageWriter`s pass param validation
            // whether or not the sim's `SimActsPlugin` (the READER side) is present
            // (`bevy-traps.md` #4 — a `MessageWriter<M>` needs its `Messages<M>`
            // buffer). `add_message` is IDEMPOTENT (it no-ops if the buffer already
            // exists), so this coexists with E10's `BattleSimPlugin` also adding them
            // via `SimActsPlugin` — this slice still adds NO dispatch SYSTEM (it emits
            // only; the sim consumes).
            .add_message::<FireRequested>()
            .add_message::<MoveRequested>()
            .add_message::<SetStanceRequested>()
            .add_message::<SetAimingRequested>()
            .add_message::<SetFacingRequested>()
            // GTW-251 — register the presenter-defined `HighlightRequest` buffer so the
            // emitter's `MessageWriter<HighlightRequest>` passes param validation even
            // headlessly (a `MessageWriter<M>` needs its `Messages<M>` buffer,
            // `bevy-traps.md` #4). `add_message` is IDEMPOTENT, so this coexists with the
            // presenter's `TopDownRendererPlugin` also adding the same buffer (the reader
            // side) — the `*Requested` precedent above.
            .add_message::<HighlightRequest>()
            .add_systems(
                Update,
                (
                    pick_hovered_cell,
                    emit_highlight_request.after(pick_hovered_cell),
                )
                    .in_set(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-255 — set the INITIAL selection once: when the battle is live with a
            // player faction and NOTHING is selected yet, auto-select the deterministic
            // player-faction ganger (lowest `(level, y, x)` cell). Gated to a live battle
            // WITH the player faction it reads (a `Res<T>` of an absent resource fails
            // param validation, `bevy-traps.md` #1; both are battle-scoped). Ordered
            // `.before(left_click_act)` in the SAME band as the click selection writer so
            // the same update's `sync_fire_mode_on_select` + `update_selection_highlight`
            // (both `.after(left_click_act)`) react to the new selection exactly as for a
            // click. It only FILLS an empty selection, so it never fights a player click
            // (which writes the selection later the same update / on a later one).
            .add_systems(
                Update,
                auto_select_first_player_ganger
                    .in_set(InputSystems::Gather)
                    .before(left_click_act)
                    .run_if(
                        resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>),
                    ),
            )
            // GTW-238 — the ONE disambiguated left-click decision (FIRE -> SELECT ->
            // MOVE -> CLEAR) + the right-click turn-to-face surface, REPLACING the
            // GTW-225/227 `select_on_click` + `fire_on_click` race. Both are gated to a
            // live battle WITH the player faction + the occupancy / mouse / tuning the
            // decision reads: a `Res<T>` of an absent resource fails param validation
            // (`bevy-traps.md` #1), and `PlayerFaction` / `OccupancyGrid` /
            // `CombatTuning` are all battle-scoped (inserted on setup, removed on
            // teardown), `ButtonInput<MouseButton>` is absent under `MinimalPlugins`
            // (no `InputPlugin`). `right_click_turn_to_face` is gated the SAME way
            // (it reads a subset, but the uniform gate keeps both inert pre-battle).
            //
            // Ordered `.before(pick_hovered_cell)` (`bevy-traps.md` #3): both read the
            // `HoveredCell` BEFORE this update's `pick_hovered_cell` rewrites it — i.e.
            // they act on the cell resolved last update. For a click the cursor is
            // effectively stationary across one frame, so the one-frame read is exact,
            // and it makes the consume->resolve order DETERMINISTIC (no flaky
            // pick-vs-consumer race). Both run `.before(dispatch_act_intents)` so the
            // ONE drain sees this update's pushes.
            .add_systems(
                Update,
                (left_click_act, right_click_turn_to_face)
                    .in_set(InputSystems::Gather)
                    .before(pick_hovered_cell)
                    .before(dispatch_act_intents)
                    .run_if(
                        resource_exists::<BattleInProgress>
                            .and(resource_exists::<OccupancyGrid>)
                            .and(resource_exists::<ButtonInput<MouseButton>>)
                            .and(resource_exists::<CombatTuning>)
                            .and(resource_exists::<PlayerFaction>),
                    ),
            )
            .add_systems(
                Update,
                update_selection_highlight
                    .in_set(InputSystems::Gather)
                    .after(left_click_act)
                    .run_if(
                        resource_exists::<BattleInProgress>.and(resource_exists::<OccupancyGrid>),
                    ),
            )
            // 222b: on a fresh selection, default `SelectedFireMode` to the picked
            // weapon's `FireMode::single()` (AC1). Runs after `left_click_act` so it
            // observes the same update's selection.
            .add_systems(
                Update,
                sync_fire_mode_on_select
                    .in_set(InputSystems::Gather)
                    .after(left_click_act)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // S8 + 222b keyboard press surface: reads the loaded `Keybinds` (so it is
            // gated on that resource existing too) and pushes intents. The act-bearing
            // keys (`posture_keys` / `fire_mode_cycle_key`, 222b) push the same seam
            // 222a's no-act keys (`level_keys` / `select_clear_key`) do.
            .add_systems(
                Update,
                (
                    level_keys,
                    select_clear_key,
                    posture_keys,
                    fire_mode_cycle_key,
                )
                    .in_set(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<Keybinds>)),
            )
            // The ONE intent drain — after EVERY intent writer (the keyboard keys +
            // GTW-238's left-click decision + right-click turn surface) so it sees this
            // update's pushes; the 222c button writers (in `gdtf_app`'s `Update`) also
            // feed it.
            .add_systems(
                Update,
                dispatch_act_intents
                    .in_set(InputSystems::Gather)
                    .after(level_keys)
                    .after(select_clear_key)
                    .after(posture_keys)
                    .after(fire_mode_cycle_key)
                    .after(left_click_act)
                    .after(right_click_turn_to_face)
                    .run_if(resource_exists::<BattleInProgress>),
            );

        // The data-driven keybind table loads the GTW-136 RON way. `init_ron_asset`
        // PANICS at registration without an `AssetServer` (no `Assets<T>` machinery),
        // so it — and the load/resolve chain — is gated on the asset stack being
        // present (the presenter's `tile_roles` precedent, `bevy-traps.md` #1). Under
        // `DefaultPlugins` (the app + the `AssetServer` harness) it runs for real;
        // under `MinimalPlugins` it is skipped entirely (no load, no panic) and a test
        // inserts `Keybinds` directly.
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<Keybinds>()
                .add_systems(Startup, load_keybinds)
                .add_systems(
                    Update,
                    resolve_keybinds.run_if(
                        resource_exists::<KeybindsHandle>.and(not(resource_exists::<Keybinds>)),
                    ),
                );
        }
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

/// EMITS a presenter-owned [`HighlightRequest`] reflecting the picked [`HoveredCell`].
///
/// The INPUT half of the GTW-251 message-driven hover-highlight: instead of DRAWING the
/// reticle (the old `update_hover_highlight`, now the presenter's
/// [`draw_highlight_on_request`](gdtf_battle_presenter::draw_highlight_on_request)), it
/// writes one [`HighlightRequest`]`(`[`*hovered`](HoveredCell)`)` every update it runs —
/// [`Some(cell)`](Some) when a cell is hovered, [`None`] when nothing is. The presenter
/// LISTENS for that message and moves/shows/hides the one reticle sprite, so the drawn
/// highlight ALWAYS matches [`HoveredCell`].
///
/// Emitting EVERY update (not only on `Changed<HoveredCell>`) keeps the contract simple
/// and the highlight in lockstep: the presenter's draw acts on the latest request, so a
/// per-frame re-emit of the unchanged cell is a no-op move; and a once-only-on-change
/// emit could miss the first draw if the picker resolved the cell before the presenter's
/// reader was ready. Both are correct per the contract; the per-frame emit is the most
/// robust.
///
/// Param-only (`bevy-traps.md` #7): a [`Res<HoveredCell>`](HoveredCell) read + a
/// [`MessageWriter<HighlightRequest>`](bevy::ecs::message::MessageWriter) write, no
/// `&mut World`. Runs `.after(pick_hovered_cell)` so it emits the SAME update's resolved
/// [`HoveredCell`], under the same `BattleInProgress` gate so it is inert pre-battle.
pub fn emit_highlight_request(
    hovered: Res<HoveredCell>,
    mut requests: MessageWriter<HighlightRequest>,
) {
    requests.write(HighlightRequest(**hovered));
}

#[cfg(test)]
mod tests {
    use gdtf_battle_presenter::cell_to_world;

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

    /// A marker pushed into the shared order log by the band probes — distinguishes the
    /// input band from the sim band so the test can read their relative run order.
    ///
    /// Test-only; the framework plumbing carve-out — a tiny enum the probe systems push
    /// to record which band ran.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ProbeBand {
        /// Pushed by the probe in [`InputSystems::Gather`].
        Input,
        /// Pushed by the probe in [`SimSystems::Simulate`].
        Sim,
    }

    /// The shared order-recording resource AC2's two probe systems append to.
    ///
    /// A `Vec` log (test-only framework plumbing) recording the order the input-band and
    /// sim-band probes ran within one `Update`, so the assert can read input-before-sim.
    #[derive(bevy::prelude::Resource, Default)]
    struct OrderLog(Vec<ProbeBand>);

    /// Probe in the input band: records that [`InputSystems::Gather`] ran.
    fn probe_input(mut log: bevy::prelude::ResMut<OrderLog>) {
        log.0.push(ProbeBand::Input);
    }

    /// Probe in the sim band: records that [`SimSystems::Simulate`] ran.
    fn probe_sim(mut log: bevy::prelude::ResMut<OrderLog>) {
        log.0.push(ProbeBand::Sim);
    }

    /// AC2 — the input band runs BEFORE the sim band within one `Update` (the
    /// behavioral order pin).
    ///
    /// Builds a headless `MinimalPlugins` app, adds the real [`GdtfBattleInputPlugin`]
    /// (which `configure_sets(Update, InputSystems::Gather.before(SimSystems::Simulate))`),
    /// and registers two test-owned probes against a shared [`OrderLog`]: one
    /// `.in_set(InputSystems::Gather)` and one `.in_set(SimSystems::Simulate)`. The test
    /// owns the sim set's existence via its own `configure_sets(Update,
    /// SimSystems::Simulate)` (the contract's sanctioned "configure it itself" option) so
    /// no resource-requiring sim system is dragged in. After ONE `app.update()`, the log
    /// shows the input marker BEFORE the sim marker.
    ///
    /// Pin-discriminating: remove the plugin's
    /// `InputSystems::Gather.before(SimSystems::Simulate)` configure and the two bands
    /// become unordered (`bevy-traps.md` #3), so this exact order is no longer
    /// guaranteed and the assert can fail. Driven from the test body (`bevy-traps.md` #7
    /// carve-out).
    #[test]
    fn input_band_runs_before_sim_band() {
        use bevy::prelude::{App, IntoScheduleConfigs, MinimalPlugins, Update};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GdtfBattleInputPlugin)
            .init_resource::<OrderLog>()
            // The test owns the sim set's existence (the sanctioned alternative to adding
            // `OccupancyMaintenancePlugin`, whose member systems would need battle-scoped
            // resources). `configure_sets` accumulates (`bevy-traps.md` #5), so this
            // composes with the plugin's `InputSystems::Gather.before(SimSystems::Simulate)`.
            .configure_sets(Update, SimSystems::Simulate)
            .add_systems(Update, probe_input.in_set(InputSystems::Gather))
            .add_systems(Update, probe_sim.in_set(SimSystems::Simulate));

        app.update();

        let log = &app.world().resource::<OrderLog>().0;
        let input_at = log.iter().position(|b| *b == ProbeBand::Input);
        let sim_at = log.iter().position(|b| *b == ProbeBand::Sim);
        let (Some(input_at), Some(sim_at)) = (input_at, sim_at) else {
            assert_eq!(
                (input_at.is_some(), sim_at.is_some()),
                (true, true),
                "both band probes must have run once in the single update",
            );
            return;
        };
        assert!(
            input_at < sim_at,
            "the input band must run BEFORE the sim band within one Update \
             (log: {log:?})",
        );
    }
}

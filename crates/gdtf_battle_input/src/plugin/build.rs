//! The input plugin (GTW-221 / GTW-225 / GTW-238 / GTW-251 / GTW-255 / GTW-259): wires
//! cursor->cell picking, the hover-highlight emitter, ganger selection, level cycling, the
//! data-driven keybinds, the shared act-intent seam, and the gamepad software cursor.

use bevy::{ecs::message::Messages, prelude::*, window::CursorMoved};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::{
    FireTargetHighlight, GamepadCursorMoved, HighlightRequest, PathPreview,
};
#[cfg(debug_assertions)]
use gdtf_battle_presenter::{ReachableCells, ReachableOverlayEnabled};
// GTW-450 — FloorCostGrid is consumed ONLY by the DEBUG-gated reachable-overlay populate fn
// (`register_reachable_overlay_population`), so import it only under `#[cfg(debug_assertions)]`
// to keep the release build from naming an unused symbol.
#[cfg(debug_assertions)]
use gdtf_battle_sim::FloorCostGrid;
use gdtf_battle_sim::{
    BattleInProgress, OccupancyGrid, PlayerFaction, SquadVisibility, VerticalLinkGraph,
    acts::{
        EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
        ExitEmplacementRequested, FireRequested, MeleeRequested, MoveRequested, OpenDoorRequested,
        ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
        ShoveRequested, StabilizeDownedRequested,
    },
    occupancy_sync::SimSystems,
    setup_battle_on_request,
    tuning::CombatTuning,
};

// GTW-450 — the reachable-overlay POPULATE system + its presenter-owned flag are DEBUG-only
// (C1); imported only under `#[cfg(debug_assertions)]` so the release build never names them.
#[cfg(debug_assertions)]
use crate::selection::populate_reachable_overlay;
use crate::{
    InputSystems,
    fire_mode::{SelectedFireMode, sync_fire_mode_on_select},
    gamepad::{
        ActivePointer, GamepadCursor, emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn,
        mouse_reclaims_pointer, move_gamepad_cursor,
    },
    intent::{PendingActIntent, dispatch_act_intents},
    keybinds::{
        Keybinds, KeybindsHandle, load_keybinds, redrive_keybinds_on_asset_event, resolve_keybinds,
    },
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    picking::{InspectTarget, emit_highlight_request, pick_hovered_cell},
    selection::{
        PathPreviewTarget, SelectedShooter, auto_select_first_player_ganger, left_click_act,
        populate_fire_target, populate_path_preview, reset_move_target_on_fire_mode_change,
        right_click_turn_to_face, update_selection_highlight,
    },
};

/// Marker resource the [`GdtfBattleInputPlugin`] inserts on `build`.
///
/// Its presence in the world is the test-observable proof that the input plugin's
/// `build` actually ran inside the real scene stack (AC1 asserts it present after the
/// harness descends to the battle). A framework type (`Resource`), exempt from no-bare-types.
#[derive(Resource)]
pub struct GdtfBattleInputActive;

/// The input plugin: cursor->cell picking + hover-highlight (S7), ganger selection,
/// level cycling, the data-driven keybinds, and the shared act-intent seam (S8).
///
/// Added by `gdtf_app`'s `GameBattleScapeScenePlugin` BESIDE
/// `BattlePresenterPlugin::default()`, so its `build` runs when the scene plugins register.
/// On `build` it:
///
/// - inserts the [`GdtfBattleInputActive`] marker and initialises the [`InspectTarget`] (S7),
///   [`SelectedShooter`], [`SelectedFireMode`] (222b), and [`PendingActIntent`] resources,
///   and registers the `*Requested` message buffers the drain emits;
/// - registers the S7 cursor picker ([`pick_hovered_cell`]) + the GTW-251 highlight EMITTER
///   ([`emit_highlight_request`]) and the [`HighlightRequest`] buffer;
/// - registers the GTW-238 unified left-click decision ([`left_click_act`]) + the right-click
///   turn-to-face surface ([`right_click_turn_to_face`]), the GTW-255 battle-start auto-select
///   ([`auto_select_first_player_ganger`]), the selection highlight
///   ([`update_selection_highlight`]), the 222b fire-mode default-on-select
///   ([`sync_fire_mode_on_select`]), the keyboard press surface, and the ONE intent drain
///   ([`dispatch_act_intents`]); and
/// - loads the data-driven keybind table ([`load_keybinds`] / [`resolve_keybinds`]) via the
///   GTW-136 `RonAsset<T>` path, gated on an [`AssetServer`] so a `MinimalPlugins` headless
///   app no-ops (`bevy-traps.md` #1).
///
/// All live-battle systems are gated `run_if(resource_exists::<BattleInProgress>)` so the
/// input layer is inert pre-battle (AC11). The GTW-238 click decision additionally gates on
/// [`PlayerFaction`] + the occupancy / mouse / tuning it reads.
///
/// Ordering (`bevy-traps.md` #3): the highlight emitter runs `.after(pick_hovered_cell)` and
/// the selection highlight `.after(left_click_act)` so each observes the SAME update's
/// resolved cell/selection. [`left_click_act`] / [`right_click_turn_to_face`] run
/// `.before(pick_hovered_cell)` (the cell resolved last update) and
/// `.before(dispatch_act_intents)` (the drain). The intent drain runs `.after` EVERY intent
/// WRITER (the keyboard press systems + the click decision) so it drains the same update's
/// pushes — and the 222c `gdtf_app` buttons that push the same seam run in `Update` upstream
/// of it.
pub struct GdtfBattleInputPlugin;

/// The shared run-condition for the click / gamepad ACT decision systems: a live
/// battle WITH the player faction AND the occupancy / mouse-button / tuning the
/// decision reads (`bevy-traps.md` #1). Factored into one combinator so the mouse
/// ([`left_click_act`] / [`right_click_turn_to_face`]) and gamepad
/// ([`gamepad_click_act`] / [`gamepad_turn`]) registrations share the identical gate
/// without restating the five-resource chain at each `run_if` (it also keeps the
/// plugin `build` body under clippy's `too_many_lines`).
fn battle_act_gate() -> impl SystemCondition<()> {
    resource_exists::<BattleInProgress>
        .and_then(resource_exists::<OccupancyGrid>)
        .and_then(resource_exists::<ButtonInput<MouseButton>>)
        .and_then(resource_exists::<CombatTuning>)
        .and_then(resource_exists::<PlayerFaction>)
        // GTW-356 — the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
        // link-tile non-target gate, via `LeftClickReads`), so a focused headless harness that
        // opens `BattleInProgress` WITHOUT routing through `setup_battle` (which seeds it) keeps
        // the click systems inert rather than panicking the `Res` param validation
        // (`bevy-traps.md` #1). In the real app `setup_battle` inserts it, so the click decision
        // runs exactly when a battle is live.
        .and_then(resource_exists::<VerticalLinkGraph>)
}

impl Plugin for GdtfBattleInputPlugin {
    #[expect(
        clippy::too_many_lines,
        reason = "plugin build: each line registers a system; extracting further improves nothing"
    )]
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
        // whether or not a battle is live.
        //
        // GTW play-test wave 3 (A2) — ALSO anchor the input band AFTER the sim's
        // `setup_battle_on_request` (the sim crate registers it `.before(SimSystems::Simulate)`
        // but, living downstream of the input crate, CANNOT itself reference `InputSystems`).
        // Without this edge the setup system and the input band were both merely
        // `.before(SimSystems::Simulate)` with NO order BETWEEN them, so on the battle's first
        // frame the GTW-255 `auto_select_first_player_ganger` (in this band, gated
        // `run_if(resource_exists::<PlayerFaction>)`) could run BEFORE setup inserted
        // `PlayerFaction` + spawned the gangers — its run-condition false, nothing selected, the
        // status panel stuck on "No ganger selected". The edge forces an apply-deferred sync
        // point so setup's `Commands` (the `PlayerFaction` insert + the ganger spawns) are
        // applied and queryable before the input band runs that same `Update` (`bevy-traps.md`
        // #3). The edge is a no-op when `setup_battle_on_request` is absent (the input crate's
        // own `MinimalPlugins` unit tests, which do not add `BattleSimPlugin`).
        app.configure_sets(
            Update,
            InputSystems::Gather
                .before(SimSystems::Simulate)
                .after(setup_battle_on_request),
        )
        .insert_resource(GdtfBattleInputActive)
        .init_resource::<InspectTarget>()
        .init_resource::<SelectedShooter>()
        .init_resource::<SelectedFireMode>()
        // GTW-358 — the route path-preview TARGET seam (the cell the preview previews TO).
        // Its `Default` is the empty target (`None` → no preview); the GTW-356 two-click flow
        // SETS it on click-1 (target select). THIS ticket only DEFINES + wires it.
        .init_resource::<PathPreviewTarget>()
        .init_resource::<PendingActIntent>()
        // GTW-259 — the gamepad software cursor + the last-moved-wins pointer arbiter.
        // `GamepadCursor` inits to its window-centre default; `ActivePointer` to `Mouse`.
        .init_resource::<GamepadCursor>()
        .init_resource::<ActivePointer>()
        // The seam EMITS these `*Requested` messages — register every buffer the ONE
        // `dispatch_act_intents` drain writes into so its `MessageWriter`s pass param
        // validation whether or not the sim's `SimActsPlugin` is present (`bevy-traps.md`
        // #4). `add_message` is IDEMPOTENT, so this coexists with E10's `BattleSimPlugin`.
        .add_message::<FireRequested>()
        .add_message::<MoveRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetFacingRequested>()
        // GTW-275 — the reload-act buffer the weapon panel's Reload intent drains into.
        .add_message::<ReloadRequested>()
        // GTW-309 — the fieldless end-turn buffer the action-bar End-Turn intent drains
        // into, so the drain's `MessageWriter<EndTurnRequested>` passes param validation
        // whether or not `SimActsPlugin` is present (`add_message` is IDEMPOTENT).
        .add_message::<EndTurnRequested>()
        // GTW-294 — the downed-act buffers the Execute / Stabilize intents drain into, so
        // the drain's `MessageWriter<ExecuteDownedRequested>` / `<StabilizeDownedRequested>`
        // pass param validation whether or not `SimActsPlugin` is present (`add_message` is
        // IDEMPOTENT, so this coexists with the sim's registration).
        .add_message::<ExecuteDownedRequested>()
        .add_message::<StabilizeDownedRequested>()
        // GTW-507 — the melee buffer the Melee intent drains into, so the drain's
        // `MessageWriter<MeleeRequested>` passes param validation whether or not `SimActsPlugin`
        // is present (`add_message` is IDEMPOTENT, so this coexists with the sim's registration).
        .add_message::<MeleeRequested>()
        // GTW-525 — the shove buffer the Shove intent drains into, so the drain's
        // `MessageWriter<ShoveRequested>` passes param validation whether or not `SimActsPlugin`
        // is present (`add_message` is IDEMPOTENT, so this coexists with the sim's registration).
        .add_message::<ShoveRequested>()
        // GTW-315 — the open-door buffer the OpenDoor intent drains into, so the drain's
        // `MessageWriter<OpenDoorRequested>` passes param validation whether or not `SimActsPlugin`
        // is present (`add_message` is IDEMPOTENT, so this coexists with the sim's registration).
        .add_message::<OpenDoorRequested>()
        // GTW-543 — the enter/exit-emplacement buffers the EnterEmplacement / ExitEmplacement
        // intents drain into, so the drain's `MessageWriter<EnterEmplacementRequested>` /
        // `<ExitEmplacementRequested>` pass param validation whether or not `SimActsPlugin` is
        // present (`add_message` is IDEMPOTENT, so this coexists with the sim's registration).
        .add_message::<EnterEmplacementRequested>()
        .add_message::<ExitEmplacementRequested>()
        // GTW-251 — register the presenter-defined `HighlightRequest` buffer so the
        // emitter's `MessageWriter<HighlightRequest>` passes param validation even
        // headlessly (`bevy-traps.md` #4). `add_message` is IDEMPOTENT.
        .add_message::<HighlightRequest>()
        // GTW-259 — register the presenter-defined `GamepadCursorMoved` buffer so the
        // gamepad-cursor emitter's `MessageWriter` passes param validation even headlessly
        // (`bevy-traps.md` #4). `add_message` is IDEMPOTENT (input→presenter, no cycle).
        .add_message::<GamepadCursorMoved>()
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
        // player-faction ganger (lowest `(level, y, x)` cell). Ordered
        // `.before(left_click_act)` so the same update's `sync_fire_mode_on_select` +
        // `update_selection_highlight` react to the new selection exactly as for a click.
        .add_systems(
            Update,
            auto_select_first_player_ganger
                .in_set(InputSystems::Gather)
                .before(left_click_act)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>),
                ),
        )
        // GTW-238 — the ONE disambiguated left-click decision (FIRE -> SELECT -> MOVE ->
        // CLEAR) + the right-click turn-to-face surface. Both gated to a live battle WITH
        // the player faction + the occupancy / mouse / tuning the decision reads
        // (`bevy-traps.md` #1). Ordered `.before(pick_hovered_cell)` (`bevy-traps.md` #3 —
        // they act on the cell resolved last update, deterministic consume->resolve order)
        // and `.before(dispatch_act_intents)` (the drain).
        .add_systems(
            Update,
            (left_click_act, right_click_turn_to_face)
                .in_set(InputSystems::Gather)
                .before(pick_hovered_cell)
                .before(dispatch_act_intents)
                .run_if(battle_act_gate()),
        )
        .add_systems(
            Update,
            update_selection_highlight
                .in_set(InputSystems::Gather)
                .after(left_click_act)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<OccupancyGrid>),
                ),
        )
        // 222b: on a fresh selection, default `SelectedFireMode` to the picked weapon's
        // `FireMode::single()` (AC1). Runs after `left_click_act` so it observes the same
        // update's selection.
        .add_systems(
            Update,
            sync_fire_mode_on_select
                .in_set(InputSystems::Gather)
                .after(left_click_act)
                .run_if(resource_exists::<BattleInProgress>),
        )
        // S8 + 222b keyboard press surface: reads the loaded `Keybinds` (so it is gated on
        // that resource existing too) and pushes intents. (The blind fire-mode-cycle key
        // was REMOVED in GTW-254 — the `gdtf_app` popup picker replaced it.) GTW-458 adds
        // `cycle_selection_keys` (the `Tab` / `Shift+Tab` Prev/Next cycle) to the same band —
        // it is NOT gated on a selection (cycling makes a first selection), unlike
        // `posture_keys`.
        .add_systems(
            Update,
            (
                level_keys,
                // GTW-521 — the full-view toggle key. Like `level_keys` it is a GLOBAL
                // presenter-view control (not gated on a selection); it reads `Keybinds` and
                // pushes `ActIntent::ToggleFullView` onto the shared seam.
                full_view_key,
                select_clear_key,
                posture_keys,
                cycle_selection_keys,
            )
                .in_set(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<Keybinds>)),
        )
        // The ONE intent drain — after EVERY intent writer (the keyboard keys + GTW-238's
        // left-click decision + right-click turn surface) so it sees this update's pushes;
        // the 222c button writers (in `gdtf_app`'s `Update`) also feed it.
        .add_systems(
            Update,
            dispatch_act_intents
                .in_set(InputSystems::Gather)
                .after(level_keys)
                // GTW-521 — after the full-view toggle key so a toggle queued this update is
                // drained this update (the same-frame guarantee, `bevy-traps.md` #3).
                .after(full_view_key)
                .after(select_clear_key)
                .after(posture_keys)
                // GTW-458 — after the Tab/Shift+Tab cycle key so a cycle queued this update is
                // drained this update (the same-frame guarantee, `bevy-traps.md` #3).
                .after(cycle_selection_keys)
                .after(left_click_act)
                .after(right_click_turn_to_face)
                // GTW-259 — also after the gamepad act writers so the drain sees a
                // South / East push the same update.
                .after(gamepad_click_act)
                .after(gamepad_turn)
                .run_if(resource_exists::<BattleInProgress>),
        );

        // GTW-259 — the gamepad software cursor + act surfaces + edge-pan emitter. Extracted
        // into a helper to keep `build` under the `too_many_lines` lint (the presenter's
        // `register_*` extraction precedent). System-ordering edges by fn reference compose
        // across `add_systems` calls, so this helper's systems still order correctly.
        register_gamepad_systems(app);

        // GTW-358 — the route path-preview POPULATE system (calls `find_path` for the selected
        // ganger → the `PathPreviewTarget`; extracted to keep `build` under the
        // `too_many_lines` lint).
        register_path_preview_population(app);

        // GTW-450 — the reachable-range overlay POPULATE system is DEBUG-only (C1): it exists
        // solely to fill the debug overlay's read-seam, so it compiles only under
        // `#[cfg(debug_assertions)]`. A release build excludes it (the move feedback is the
        // click-to-target route preview alone, C4). Calls `reachable_within` for the selected
        // ganger; extracted to keep `build` under the `too_many_lines` lint.
        #[cfg(debug_assertions)]
        register_reachable_overlay_population(app);

        // GTW-371 — the fire-target highlight POPULATE system (decides the fireable-enemy verdict
        // on the hovered cell + computes the `mode_tu_cost`; extracted to keep `build` under the
        // `too_many_lines` lint).
        register_fire_target_population(app);

        // The data-driven keybind table loads the GTW-136 RON way. `init_ron_asset` PANICS at
        // registration without an `AssetServer`, so it — and the load/resolve chain — is gated
        // on the asset stack being present (`bevy-traps.md` #1). Under `DefaultPlugins` it runs
        // for real; under `MinimalPlugins` it is skipped (no load, no panic).
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<Keybinds>()
                .add_systems(Startup, load_keybinds)
                .add_systems(
                    Update,
                    resolve_keybinds.run_if(
                        resource_exists::<KeybindsHandle>
                            .and_then(not(resource_exists::<Keybinds>)),
                    ),
                )
                // GTW-533: the LIVE keybind hot-reload — overwrites the resident Keybinds
                // resource on a `core_tuning/keybinds.tuning.ron` edit, mirroring the game's
                // combat-tuning redrive. Ungated (it self-guards on its Optional borrows,
                // `bevy-traps.md` #1), so a live `.ron` edit re-binds keys with NO restart. Its
                // `Messages<AssetEvent<RonAsset<Keybinds>>>` buffer is registered by the
                // `init_ron_asset::<Keybinds>()` above, so the MessageReader validates
                // (`bevy-traps.md` #4).
                .add_systems(Update, redrive_keybinds_on_asset_event);
        }
    }
}

/// Registers the GTW-259 gamepad systems into [`InputSystems::Gather`]: the software-cursor
/// drive + the pointer arbitration, the South / East act surfaces, and the edge-pan emitter.
///
/// Extracted from [`GdtfBattleInputPlugin::build`](GdtfBattleInputPlugin) to keep it under the
/// `too_many_lines` lint (the presenter's `register_*` extraction precedent). Every system is
/// battle-gated (`bevy-traps.md` #1) and `InputSystems::Gather`-banded:
///
/// - [`move_gamepad_cursor`] steers the [`GamepadCursor`] by the LEFT stick and claims
///   [`ActivePointer::Gamepad`] past the deadzone; ordered `.before(pick_hovered_cell)`
///   (`bevy-traps.md` #3) so the generalized picker projects THIS update's cursor.
/// - [`mouse_reclaims_pointer`] flips back to [`ActivePointer::Mouse`] on a [`CursorMoved`]
///   message (last-moved-wins); additionally gated on its `Messages<CursorMoved>` buffer so
///   its [`MessageReader`](bevy::ecs::message::MessageReader) validates under `MinimalPlugins`.
/// - [`gamepad_click_act`] (South) + [`gamepad_turn`] (East) reuse the SHARED decision the
///   mouse uses and the SAME [`PendingActIntent`] seam, ordered `.before(pick_hovered_cell)`
///   and `.before(dispatch_act_intents)`.
/// - [`emit_gamepad_cursor_move`] writes [`GamepadCursorMoved`] for the presenter's edge-pan
///   when the gamepad is the active pointer.
fn register_gamepad_systems(app: &mut App) {
    app.add_systems(
        Update,
        move_gamepad_cursor
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        mouse_reclaims_pointer.in_set(InputSystems::Gather).run_if(
            resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<CursorMoved>>),
        ),
    )
    .add_systems(
        Update,
        (gamepad_click_act, gamepad_turn)
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .before(dispatch_act_intents)
            .run_if(battle_act_gate()),
    )
    .add_systems(
        Update,
        emit_gamepad_cursor_move
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Registers the GTW-358 route path-preview POPULATE system into [`InputSystems::Gather`], plus
/// the GTW-379 FIRE→MOVE move-target RESET that runs `.before` it.
///
/// [`populate_path_preview`] reads the current [`SelectedShooter`] + the [`PathPreviewTarget`]
/// and fills the presenter-owned [`PathPreview`](gdtf_battle_presenter::PathPreview) the SAME
/// way [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) plans a route (the
/// visibility-gated `PlanningView` over the squad fog + `find_path`), exposing
/// [`Path::total`](gdtf_battle_sim::Path::total) — the §48 cost GTW-355 charges. Ordered
/// `.after(left_click_act)` (so it reads the same update's selection) and
/// `.after(auto_select_first_player_ganger)` (so the battle-start auto-select can preview a
/// route on the first frame, exactly as a click would). The TARGET itself is set by the GTW-356
/// two-click flow (click-1).
///
/// Gated on the live battle WITH every grid the route reads — the [`OccupancyGrid`], the
/// [`VerticalLinkGraph`], the [`SquadVisibility`] fog, and the [`CombatTuning`] — AND the
/// presenter-`init_resource`-d [`PathPreview`](gdtf_battle_presenter::PathPreview): a focused
/// input-only harness opens `BattleInProgress` WITHOUT a presenter plugin (so no `PathPreview`),
/// so without that guard the
/// `ResMut<PathPreview>` param would panic validation (`bevy-traps.md` #1). In the real app
/// `setup_battle` inserts the grids + the presenter `init_resource`s `PathPreview`, so the
/// preview populates exactly when a battle is live. Extracted from
/// [`GdtfBattleInputPlugin::build`](GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_gamepad_systems` precedent).
///
/// GTW-379 — [`reset_move_target_on_fire_mode_change`] runs in the same band, ordered
/// `.before(populate_path_preview)`, so when the player engages the fire-mode toggle (a
/// `Changed<`[`SelectedFireMode`]`>`) it clears [`PathPreviewTarget`] and the populate system
/// then writes the empty [`PathPreview`](gdtf_battle_presenter::PathPreview) the SAME update —
/// hiding + resetting the stale move path on the FIRE→MOVE switch. It only reads the always-present
/// [`SelectedFireMode`] / [`PathPreviewTarget`] (both `init_resource`-d above), so it needs only
/// the `BattleInProgress` gate — NOT the grid/`PathPreview` gate the route populate needs.
fn register_path_preview_population(app: &mut App) {
    app.add_systems(
        Update,
        reset_move_target_on_fire_mode_change
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            // CRITICAL — `.after(sync_fire_mode_on_select)`: that system writes `SelectedFireMode`
            // (the on-select / GTW-376 weapon-arrival auto-default) within this same update. The
            // reset must observe that write IN ORDER, so its `last_run` advances PAST it; otherwise
            // (running before it) the reset would see the auto-default's change on the NEXT update
            // — with the selection no longer changed — and wrongly clear a move target the player
            // set that frame (the GTW-356 two-click re-target regression). It still runs
            // `.before(populate_path_preview)` so a real switch clears the preview the same update.
            .after(sync_fire_mode_on_select)
            .before(populate_path_preview)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        populate_path_preview
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<VerticalLinkGraph>)
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<PathPreview>),
            ),
    );
}

/// Registers the GTW-371 fire-target highlight POPULATE system into [`InputSystems::Gather`].
///
/// [`populate_fire_target`] reads the current [`SelectedShooter`] + [`SelectedFireMode`] + the
/// hovered cell ([`InspectTarget`](crate::InspectTarget)) and fills the presenter-owned
/// [`FireTargetHighlight`](gdtf_battle_presenter::FireTargetHighlight) when the hover is a
/// fireable ENEMY (the SAME FIRE-rung conditions [`left_click_act`] gates fire on, plus the
/// GTW-346 fog gate), exposing the [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) the shot
/// would charge. Ordered `.after(left_click_act)` (so it reads the same update's selection) and
/// `.after(auto_select_first_player_ganger)` (so the battle-start auto-select can show the
/// affordance on the first frame).
///
/// Gated on the live battle WITH the resources the verdict reads as a hard `Res` — the
/// [`OccupancyGrid`] (the occupant lookup), the [`CombatTuning`] (the aim premium), the
/// [`PlayerFaction`] (the friend/foe gate) — AND the presenter-`init_resource`-d
/// [`FireTargetHighlight`](gdtf_battle_presenter::FireTargetHighlight): a focused input-only
/// harness opens `BattleInProgress` WITHOUT a presenter plugin (so no `FireTargetHighlight`), so
/// without that guard the `ResMut<FireTargetHighlight>` param would panic validation
/// (`bevy-traps.md` #1). The [`SquadVisibility`] fog is read as an `Option` (FAIL-CLOSED on
/// absence), so it is NOT in the gate. Extracted from
/// [`GdtfBattleInputPlugin::build`](GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_path_preview_population` precedent).
fn register_fire_target_population(app: &mut App) {
    app.add_systems(
        Update,
        populate_fire_target
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            // Reads `InspectTarget`'s LIVE hovered cell, so it runs `.before(pick_hovered_cell)`
            // (the click-decision precedent, `bevy-traps.md` #3): it acts on the cell resolved
            // last update, the deterministic consume->resolve order — and a headless harness that
            // injects `InspectTarget` directly has it read before the camera-less picker clobbers
            // it to `None`.
            .before(pick_hovered_cell)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<PlayerFaction>)
                    .and_then(resource_exists::<FireTargetHighlight>),
            ),
    );
}

/// Registers the GTW-387 / GTW-450 reachable-range DEBUG overlay POPULATE system into
/// [`InputSystems::Gather`].
///
/// DEBUG-ONLY (GTW-450 C1): this fn — and every item it names — compiles only under
/// `#[cfg(debug_assertions)]`; a release build excludes it. The system additionally
/// `run_if`s the presenter-owned [`ReachableOverlayEnabled`] flag VALUE (the C3 runtime
/// opt-in), so even in a debug build it is INERT unless `GDTF_DEBUG_REACHABLE_OVERLAY` was
/// set truthy at startup — no overlay populates by default (C3 / C4).
///
/// [`populate_reachable_overlay`] reads the current [`SelectedShooter`] and its
/// `(`[`Position`](gdtf_battle_sim::Position)`,` [`Tu`](gdtf_battle_sim::Tu)`,`
/// [`Faction`](gdtf_battle_sim::Faction)`)` and fills the presenter-owned
/// [`ReachableCells`](gdtf_battle_presenter::ReachableCells) by calling
/// [`reachable_within`](gdtf_battle_sim::reachable_within) — the SAME visibility-gated
/// `PlanningView` construction the path-preview and `dispatch_move` use. Ordered
/// `.after(left_click_act)` and `.after(auto_select_first_player_ganger)` so it observes
/// the same update's selection. It recomputes every Update; writes only on a change (the
/// `!=` guard, the `populate_path_preview` precedent).
///
/// Gated on the live battle WITH every grid the flood reads (`OccupancyGrid`,
/// `VerticalLinkGraph`, `SquadVisibility`, `CombatTuning`, `FloorCostGrid`) AND the
/// presenter-`init_resource`-d [`ReachableCells`](gdtf_battle_presenter::ReachableCells):
/// a focused input-only harness opens `BattleInProgress` WITHOUT a presenter plugin (so no
/// `ReachableCells`), so without that guard the `ResMut<ReachableCells>` param would panic
/// validation (`bevy-traps.md` #1). Extracted from
/// [`GdtfBattleInputPlugin::build`](GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_path_preview_population` precedent).
#[cfg(debug_assertions)]
fn register_reachable_overlay_population(app: &mut App) {
    app.add_systems(
        Update,
        populate_reachable_overlay
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<VerticalLinkGraph>)
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<FloorCostGrid>)
                    .and_then(resource_exists::<ReachableCells>)
                    // GTW-450 C3 — the runtime opt-in: populate only when the flag is true.
                    .and_then(reachable_overlay_enabled),
            ),
    );
}

/// Run-condition: whether the reachable-range DEBUG overlay is enabled this process
/// (GTW-450 C3) — reads the presenter-owned [`ReachableOverlayEnabled`] flag VALUE.
/// DEBUG-only.
///
/// `Option<Res<…>>` (fail-closed if absent) so the populate system stays inert unless the
/// presenter seeded the flag AND it is `true`. The presenter's `build` inserts it; a
/// focused input-only harness sets the RESOURCE directly to exercise on/off (it must never
/// touch process-global env — the flaky-tests rule).
#[cfg(debug_assertions)]
fn reachable_overlay_enabled(flag: Option<Res<ReachableOverlayEnabled>>) -> bool {
    flag.is_some_and(|flag| **flag)
}

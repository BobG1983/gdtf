//! The input plugin (GTW-221 / GTW-225 / GTW-238 / GTW-251 / GTW-255 / GTW-259): wires
//! cursor->cell picking, the hover-highlight emitter, ganger selection, level cycling, the
//! data-driven keybinds, the shared act-intent seam, and the gamepad software cursor.

use bevy::{ecs::message::Messages, prelude::*, window::CursorMoved};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::{GamepadCursorMoved, HighlightRequest};
use gdtf_battle_sim::{
    BattleInProgress, OccupancyGrid, PlayerFaction,
    acts::{
        FireRequested, MoveRequested, ReloadRequested, SetAimingRequested, SetFacingRequested,
        SetStanceRequested,
    },
    occupancy_sync::SimSystems,
    setup_battle_on_request,
    tuning::CombatTuning,
};

use crate::{
    InputSystems,
    fire_mode::{SelectedFireMode, sync_fire_mode_on_select},
    gamepad::{
        ActivePointer, GamepadCursor, emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn,
        mouse_reclaims_pointer, move_gamepad_cursor,
    },
    intent::{PendingActIntent, dispatch_act_intents},
    keybinds::{Keybinds, KeybindsHandle, load_keybinds, resolve_keybinds},
    keyboard::{level_keys, posture_keys, select_clear_key},
    picking::{HoveredCell, emit_highlight_request, pick_hovered_cell},
    selection::{
        SelectedShooter, auto_select_first_player_ganger, left_click_act, right_click_turn_to_face,
        update_selection_highlight,
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
/// - inserts the [`GdtfBattleInputActive`] marker and initialises the [`HoveredCell`] (S7),
///   [`SelectedShooter`], [`SelectedFireMode`] (222b), and [`PendingActIntent`] resources,
///   and registers the five `*Requested` message buffers the drain emits;
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
        .init_resource::<HoveredCell>()
        .init_resource::<SelectedShooter>()
        .init_resource::<SelectedFireMode>()
        .init_resource::<PendingActIntent>()
        // GTW-259 — the gamepad software cursor + the last-moved-wins pointer arbiter.
        // `GamepadCursor` inits to its window-centre default; `ActivePointer` to `Mouse`.
        .init_resource::<GamepadCursor>()
        .init_resource::<ActivePointer>()
        // The seam EMITS these `*Requested` messages — register the six buffers the ONE
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
                .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>)),
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
                .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<OccupancyGrid>)),
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
        // was REMOVED in GTW-254 — the `gdtf_app` popup picker replaced it.)
        .add_systems(
            Update,
            (level_keys, select_clear_key, posture_keys)
                .in_set(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<Keybinds>)),
        )
        // The ONE intent drain — after EVERY intent writer (the keyboard keys + GTW-238's
        // left-click decision + right-click turn surface) so it sees this update's pushes;
        // the 222c button writers (in `gdtf_app`'s `Update`) also feed it.
        .add_systems(
            Update,
            dispatch_act_intents
                .in_set(InputSystems::Gather)
                .after(level_keys)
                .after(select_clear_key)
                .after(posture_keys)
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
                        resource_exists::<KeybindsHandle>.and(not(resource_exists::<Keybinds>)),
                    ),
                );
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
            resource_exists::<BattleInProgress>.and(resource_exists::<Messages<CursorMoved>>),
        ),
    )
    .add_systems(
        Update,
        (gamepad_click_act, gamepad_turn)
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
        emit_gamepad_cursor_move
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

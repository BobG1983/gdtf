//! The input plugin (GTW-221 / GTW-225 / GTW-238 / GTW-251 / GTW-255 / GTW-259): wires
//! cursor->cell picking, the hover-highlight emitter, ganger selection, level cycling, the
//! data-driven keybinds, the shared act-intent queue, and the gamepad software cursor.

use bevy::prelude::*;
use gdtf_battle_presenter::{GamepadCursorMoved, HighlightRequest, playback_caught_up};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::{PlayerFaction, setup_battle_on_request},
    occupancy_sync::SimSystems,
    prelude::{BattleInProgress, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};

#[cfg(debug_assertions)]
use super::populate_reg::register_reachable_overlay_population;
use super::{
    populate_reg::{register_fire_target_population, register_path_preview_population},
    surface_reg::{register_contextual_acts, register_gamepad_systems},
};
use crate::{
    InputSystems,
    fire_mode::{SelectedFireMode, sync_fire_mode_on_select},
    gamepad::{ActivePointer, GamepadCursor, gamepad_click_act, gamepad_turn},
    intent::{PendingActIntent, dispatch_act_intents},
    keybinds::{Keybinds, register_keybinds_hot_ron},
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    picking::{InspectTarget, emit_highlight_request, pick_hovered_cell},
    selection::{
        PathPreviewTarget, SelectedShooter, auto_select_first_player_ganger,
        clear_downed_selection, left_click_act, right_click_turn_to_face,
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
/// level cycling, the data-driven keybinds, and the shared act-intent queue (S8).
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
/// - loads the data-driven keybind table through the GTW-564 generic hot-RON load path
///   ([`register_keybinds_hot_ron`](crate::keybinds)), self-gated on an
///   [`AssetServer`](bevy::asset::AssetServer) so a `MinimalPlugins` headless app no-ops
///   (`bevy-traps.md` #1).
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
/// pushes — and the 222c `gdtf_app` buttons that push the same queue run in `Update` upstream
/// of it.
pub struct GdtfBattleInputPlugin;

/// The shared run-condition for the click / gamepad ACT decision systems: a live
/// battle WITH the player faction AND the occupancy / mouse-button / tuning the
/// decision reads (`bevy-traps.md` #1). Factored into one combinator so the mouse
/// ([`left_click_act`] / [`right_click_turn_to_face`]) and gamepad
/// ([`gamepad_click_act`] / [`gamepad_turn`]) registrations share the identical gate
/// without restating the five-resource chain at each `run_if` (it also keeps the
/// plugin `build` body under clippy's `too_many_lines`).
pub(super) fn battle_act_gate() -> impl SystemCondition<()> {
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
        reason = "GTW-727 split the keyboard registration in two (the VIEW keys stay live \
                  while the presenter catches up; the ACT keys are blocked at the push \
                  site), which pushed this over the line gate. It is one flat registration \
                  list whose per-entry comments ARE the wiring documentation; the \
                  independent surfaces are already extracted into `register_*` helpers"
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
        // GTW-358 — the route path-preview TARGET resource (the cell the preview previews TO).
        // Its `Default` is the empty target (`None` → no preview); the GTW-356 two-click flow
        // SETS it on click-1 (target select). THIS ticket only DEFINES + wires it.
        .init_resource::<PathPreviewTarget>()
        .init_resource::<PendingActIntent>()
        // GTW-259 — the gamepad software cursor + the last-moved-wins pointer arbiter.
        // `GamepadCursor` inits to its window-centre default; `ActivePointer` to `Mouse`.
        .init_resource::<GamepadCursor>()
        .init_resource::<ActivePointer>()
        // The drain EMITS these `*Requested` messages — register every buffer the ONE
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
        // GTW-571 — the CONTEXTUAL acts' `*Requested` buffers are registered by the
        // per-act `add_contextual_act::<A>()` registrar (see `register_contextual_acts`
        // below), never listed here one by one.
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
        // GTW-729 — drop a STALE selection: when the currently-selected ganger has gone Downed /
        // Dead (e.g. struck down by an enemy reaction while it was the acting unit), clear the
        // selection so it never strands on a unit that cannot act. Ordered
        // `.before(auto_select_first_player_ganger)` so the cleared selection is REFILLED with the
        // next Alive player ganger the SAME update (clear → advance).
        .add_systems(
            Update,
            clear_downed_selection
                .in_set(InputSystems::Gather)
                .before(auto_select_first_player_ganger)
                .run_if(resource_exists::<BattleInProgress>),
        )
        // GTW-255 — set the INITIAL selection once: when the battle is live with a
        // player faction and NOTHING is selected yet, auto-select the deterministic ALIVE
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
        // GTW-727 C24: `right_click_turn_to_face` is a pure act push, so it is BLOCKED
        // outright while the presenter is catching up — the turn is never even queued.
        // `left_click_act` is only PARTIALLY gated and therefore keeps running: it applies
        // two independent decisions, and only the ACT half (`decide_left_click`) is blocked
        // inside the system. The inspect-panel PIN half stays live, because pinning a
        // ganger to read its card is how a player watches the exchange that closed the gate.
        .add_systems(
            Update,
            left_click_act
                .in_set(InputSystems::Gather)
                .before(pick_hovered_cell)
                .before(dispatch_act_intents)
                .run_if(battle_act_gate()),
        )
        .add_systems(
            Update,
            right_click_turn_to_face
                .in_set(InputSystems::Gather)
                .before(pick_hovered_cell)
                .before(dispatch_act_intents)
                .run_if(battle_act_gate().and_then(playback_caught_up)),
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
        // GTW-727 C25: the two VIEW keys stay LIVE while the presenter catches up. They
        // change nothing in the world, and they are the player's means of WATCHING the
        // reaction fire that closed the gate — locking them out would remove the very
        // affordance this pacing exists to serve.
        .add_systems(
            Update,
            (level_keys, full_view_key)
                .in_set(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<Keybinds>)),
        )
        // GTW-727 C24: the act / selection keys are BLOCKED while catching up, at the PUSH
        // site — so the intent is never queued at all, rather than queued and discarded.
        .add_systems(
            Update,
            (select_clear_key, posture_keys, cycle_selection_keys)
                .in_set(InputSystems::Gather)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<Keybinds>)
                        .and_then(playback_caught_up),
                ),
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

        // GTW-571 — the CONTEXTUAL acts: configure the ONE explicitly-ordered drain set,
        // then one compile-time registration line per act (the descriptor/registrar machinery;
        // extracted to keep `build` under the `too_many_lines` lint).
        register_contextual_acts(app);

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
        // solely to fill the debug overlay's read resource, so it compiles only under
        // `#[cfg(debug_assertions)]`. A release build excludes it (the move feedback is the
        // click-to-target route preview alone, C4). Calls `reachable_within` for the selected
        // ganger; extracted to keep `build` under the `too_many_lines` lint.
        #[cfg(debug_assertions)]
        register_reachable_overlay_population(app);

        // GTW-371 — the fire-target highlight POPULATE system (decides the fireable-enemy verdict
        // on the hovered cell + computes the `mode_tu_cost`; extracted to keep `build` under the
        // `too_many_lines` lint).
        register_fire_target_population(app);

        // The data-driven keybind table loads through the GTW-564 generic hot-RON load path —
        // ONE ext call at its owning module (kick-off / gated resolve / live GTW-533
        // redrive), self-gated on the `AssetServer` so a `MinimalPlugins` app skips it
        // (no load, no panic — `bevy-traps.md` #1).
        register_keybinds_hot_ron(app);
    }
}

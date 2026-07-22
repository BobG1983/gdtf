//! Presenter-resource POPULATE registrations: the route path preview, the fire-target highlight,
//! and the debug-only reachable-range overlay.

use bevy::prelude::*;
use gdtf_battle_presenter::{FireTargetHighlight, PathPreview};
#[cfg(debug_assertions)]
use gdtf_battle_presenter::{ReachableCells, ReachableOverlayEnabled};
// GTW-450 — FloorCostGrid is consumed ONLY by the DEBUG-gated reachable-overlay populate fn
// (`register_reachable_overlay_population`), so import it only under `#[cfg(debug_assertions)]`
// to keep the release build from naming an unused symbol.
#[cfg(debug_assertions)]
use gdtf_battle_sim::floor::FloorCostGrid;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

// GTW-450 — the reachable-overlay POPULATE system + its presenter-owned flag are DEBUG-only
// (C1); imported only under `#[cfg(debug_assertions)]` so the release build never names them.
#[cfg(debug_assertions)]
use crate::selection::populate_reachable_overlay;
use crate::{
    InputSystems,
    fire_mode::sync_fire_mode_on_select,
    picking::pick_hovered_cell,
    selection::{
        auto_select_first_player_ganger, left_click_act, populate_fire_target,
        populate_path_preview, reset_move_target_on_fire_mode_change,
    },
};

/// Registers the GTW-358 route path-preview POPULATE system into [`InputSystems::Gather`], plus
/// the GTW-379 FIRE→MOVE move-target RESET that runs `.before` it.
///
/// [`populate_path_preview`] reads the current [`SelectedShooter`](crate::SelectedShooter) + the [`PathPreviewTarget`](crate::selection::PathPreviewTarget)
/// and fills the presenter-owned [`PathPreview`](gdtf_battle_presenter::PathPreview) the SAME
/// way [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) plans a route (the
/// visibility-gated `PlanningView` over the squad fog + `find_path`), exposing
/// [`Path::total`](gdtf_battle_sim::pathfinder::Path::total) — the §48 cost GTW-355 charges. Ordered
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
/// [`GdtfBattleInputPlugin::build`](super::GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_gamepad_systems` precedent).
///
/// GTW-379 — [`reset_move_target_on_fire_mode_change`] runs in the same band, ordered
/// `.before(populate_path_preview)`, so when the player engages the fire-mode toggle (a
/// `Changed<`[`SelectedFireMode`](crate::SelectedFireMode)`>`) it clears [`PathPreviewTarget`](crate::selection::PathPreviewTarget) and the populate system
/// then writes the empty [`PathPreview`](gdtf_battle_presenter::PathPreview) the SAME update —
/// hiding + resetting the stale move path on the FIRE→MOVE switch. It only reads the always-present
/// [`SelectedFireMode`](crate::SelectedFireMode) / [`PathPreviewTarget`](crate::selection::PathPreviewTarget) (both `init_resource`-d above), so it needs only
/// the `BattleInProgress` gate — NOT the grid/`PathPreview` gate the route populate needs.
pub(super) fn register_path_preview_population(app: &mut App) {
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
/// [`populate_fire_target`] reads the current [`SelectedShooter`](crate::SelectedShooter) + [`SelectedFireMode`](crate::SelectedFireMode) + the
/// hovered cell ([`InspectTarget`](crate::InspectTarget)) and fills the presenter-owned
/// [`FireTargetHighlight`](gdtf_battle_presenter::FireTargetHighlight) when the hover is a
/// fireable ENEMY (the SAME FIRE-rung conditions [`left_click_act`] gates fire on, plus the
/// GTW-346 fog gate), exposing the [`mode_tu_cost`](gdtf_battle_sim::magazine::mode_tu_cost) the shot
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
/// [`GdtfBattleInputPlugin::build`](super::GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_path_preview_population` precedent).
pub(super) fn register_fire_target_population(app: &mut App) {
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
/// [`populate_reachable_overlay`] reads the current [`SelectedShooter`](crate::SelectedShooter) and its
/// `(`[`Position`](gdtf_battle_sim::ganger::Position)`,` [`Tu`](gdtf_battle_sim::ganger::Tu)`,`
/// [`Faction`](gdtf_battle_sim::ganger::Faction)`)` and fills the presenter-owned
/// [`ReachableCells`](gdtf_battle_presenter::ReachableCells) by calling
/// [`reachable_within`](gdtf_battle_sim::pathfinder::reachable_within) — the SAME visibility-gated
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
/// [`GdtfBattleInputPlugin::build`](super::GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_path_preview_population` precedent).
#[cfg(debug_assertions)]
pub(super) fn register_reachable_overlay_population(app: &mut App) {
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

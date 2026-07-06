//! Hover-highlight request emission + the fog-gated reticle (GTW-251 /
//! GTW-378).

use bevy::{app::App, math::Vec2, prelude::*};
use gdtf_battle_presenter::{CellVisibility, HighlightRequest};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    occupancy::TerrainKind,
    prelude::{CellLevel, Faction, Level, OccupancyGrid},
    visibility::SquadVisibility,
};
use gdtf_test_utils::{MessageProbe, MessageProbePlugin, probed};

use super::harness::*;

/// Inserts a `SquadVisibility` whose VISIBLE (= EXPLORED) set is exactly `cells`.
fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));
}

/// GTW-378 (regression) — an ENEMY occupant on a NOT currently squad-VISIBLE (fog) cell does
/// NOT light the hover reticle (emits `HighlightRequest(None)`); making the SAME cell
/// squad-VISIBLE then emits `Some(cell)` with the `SquadVisible` verdict.
///
/// Pin-discriminating: it fails if the reticle lights over the fog-hidden enemy (the
/// info-leak the GTW-378 fix closes) AND if the visible-cell positive control never emits.
/// This is the input half of the leak fix; the inspect-panel half lives in the app's
/// `status_panel.rs`. Leaves GTW-346's blocking-wall reticle (covered by
/// `picker_emits_highlight_request_matching_hovered_cell`) untouched: only an OCCUPANT is
/// fog-gated.
#[test]
fn fog_hidden_enemy_occupant_does_not_light_the_reticle() {
    let level = Level::new(0);
    let mut app = picking_app(level);
    app.world_mut().insert_resource(OccupancyGrid::default());
    // The player squad is gang 0; the occupant is an ENEMY (gang 1).
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    let enemy = app.world_mut().spawn(Faction::new(1)).id();
    add_highlight_probe(&mut app);

    // Resolve the cell the in-grid cursor hovers, then put the enemy occupant there.
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();
    assert!(
        hovered(&app).is_some(),
        "the in-grid cursor must resolve a cell"
    );
    let Some(resolved) = hovered(&app) else {
        return;
    };
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(resolved, Some(enemy));
    }

    // FOG case — the occupant's cell is NOT squad-VISIBLE: the reticle must stay hidden (None),
    // never betraying where the fog hides the enemy.
    make_cells_visible(&mut app, &[]);
    app.world_mut()
        .resource_mut::<MessageProbe<HighlightRequest>>()
        .clear();
    app.update();
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None, CellVisibility::NotSquadVisible)],
        "a FOG-HIDDEN enemy occupant must NOT light the reticle (GTW-378 info-leak)",
    );

    // VISIBLE case (positive control) — the SAME cell is now squad-VISIBLE: the reticle lights
    // on it, carrying the SquadVisible verdict.
    make_cells_visible(&mut app, &[resolved]);
    app.world_mut()
        .resource_mut::<MessageProbe<HighlightRequest>>()
        .clear();
    app.update();
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(
            Some(resolved),
            CellVisibility::SquadVisible
        )],
        "a squad-VISIBLE enemy occupant lights the reticle (GTW-378 positive control)",
    );
}

// ---------------------------------------------------------------------------------
// GTW-251 AC1 — the picker EMITS a `HighlightRequest` matching `InspectTarget`.
// ---------------------------------------------------------------------------------

/// Adds a probe that drains `Messages<HighlightRequest>` AFTER `emit_highlight_request`
/// so it collects every request the emitter wrote this update (its own `MessageReader`
/// cursor, independent of the presenter's `draw_highlight_on_request` reader).
fn add_highlight_probe(app: &mut App) {
    // The generic GTW-576 message probe — its `Last`-schedule drain observes the same
    // update's `emit_highlight_request` write with its own reader cursor.
    app.add_plugins(MessageProbePlugin::<HighlightRequest>::default());
}

/// The requests the probe collected on the latest update (cleared each update because
/// the probe `extend`s — the test reads them right after the relevant `update()`).
fn requests(app: &App) -> Vec<HighlightRequest> {
    probed::<HighlightRequest>(app)
}

/// GTW-251 AC1 + GTW-268 — the picker emits a `HighlightRequest` matching `InspectTarget`,
/// GATED on occupancy: an in-grid cursor over a BLOCKING (or occupied) cell emits
/// `Some(that cell)`; over BARE FLOOR it emits `None` (GTW-268: gangers + objects only);
/// off-grid it emits `None`. The DRAWING is the presenter's (`tests/highlight_draw.rs`);
/// here we pin the input EMIT.
#[test]
fn picker_emits_highlight_request_matching_hovered_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);
    // GTW-268 — the emit now gates on the `OccupancyGrid`; seed one (empty) for this harness.
    app.world_mut().insert_resource(OccupancyGrid::default());
    add_highlight_probe(&mut app);

    // An in-grid cursor (shifted right + down — see the AC2 picking test for the
    // screen->world sign reasoning).
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(resolved) = cell else { return };

    // GTW-268 — over BARE FLOOR the emit is gated to None even though a cell is hovered. GTW-11 —
    // the request carries the squad-visible verdict; this harness seeds NO `SquadVisibility`, so
    // the shared `cell_squad_visible` read FAILS CLOSED to `NotSquadVisible`.
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None, CellVisibility::NotSquadVisible)],
        "a bare-floor in-grid cell must emit HighlightRequest(None) (GTW-268)",
    );

    // Mark the hovered cell BLOCKING (an object): the emit now carries Some(that cell).
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(resolved, TerrainKind::Cover);
    }
    app.world_mut()
        .resource_mut::<MessageProbe<HighlightRequest>>()
        .clear();
    app.update();
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(cell, CellVisibility::NotSquadVisible)],
        "over a blocking cell the picker must emit exactly one HighlightRequest = Some(cell) \
         (GTW-11: NotSquadVisible — no fog seeded, fail-closed)",
    );

    // Now move the cursor off-grid: InspectTarget becomes None and the emitted request
    // must follow it to None.
    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
    // Reset the probe so we read only THIS update's emit.
    app.world_mut()
        .resource_mut::<MessageProbe<HighlightRequest>>()
        .clear();
    app.update();

    assert_eq!(
        hovered(&app),
        None,
        "the off-grid cursor clears InspectTarget"
    );
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None, CellVisibility::NotSquadVisible)],
        "the picker must emit HighlightRequest(None) when nothing is hovered",
    );
}

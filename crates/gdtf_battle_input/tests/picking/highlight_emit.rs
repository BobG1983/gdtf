use bevy::{app::App, math::Vec2, prelude::*};
use cobalt_test_utils::{MessageProbe, MessageProbePlugin, probed};
use gdtf_battle_presenter::{CellVisibility, HighlightRequest};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    occupancy::TerrainKind,
    prelude::{CellLevel, Faction, Level, OccupancyGrid},
    visibility::SquadVisibility,
};

use super::harness::*;

fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));
}

#[test]
fn fog_hidden_enemy_occupant_does_not_light_the_reticle() {
    let level = Level::new(0);
    let mut app = picking_app(level);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    let enemy = app.world_mut().spawn(Faction::new(1)).id();
    add_highlight_probe(&mut app);

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

    make_cells_visible(&mut app, &[]);
    app.world_mut()
        .resource_mut::<MessageProbe<HighlightRequest>>()
        .clear();
    app.update();
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None, CellVisibility::NotSquadVisible)],
        "a FOG-HIDDEN enemy occupant must NOT light the reticle (info-leak)",
    );

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
        "a squad-VISIBLE enemy occupant lights the reticle (positive control)",
    );
}

fn add_highlight_probe(app: &mut App) {
    app.add_plugins(MessageProbePlugin::<HighlightRequest>::default());
}

fn requests(app: &App) -> Vec<HighlightRequest> {
    probed::<HighlightRequest>(app)
}

#[test]
fn picker_emits_highlight_request_matching_hovered_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);
    app.world_mut().insert_resource(OccupancyGrid::default());
    add_highlight_probe(&mut app);

    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(resolved) = cell else { return };

    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None, CellVisibility::NotSquadVisible)],
        "a bare-floor in-grid cell must emit HighlightRequest(None) ",
    );

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
         ( NotSquadVisible — no fog seeded, fail-closed)",
    );

    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
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

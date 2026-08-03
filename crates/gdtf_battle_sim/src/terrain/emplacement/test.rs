use bevy::prelude::{App, Entity, MinimalPlugins};

use super::{EmplacementOccupant, EmplacementState, EmplacementTogglePlugin, SetEmplacement};
use crate::{
    cover::HeightBand,
    ganger::StanceKind,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::TerrainCell,
    test_support::GangerEntityBuilder,
};

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(EmplacementTogglePlugin);
    app
}

fn spawn_vacant_emplacement(app: &mut App, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((TerrainCell::new(at), EmplacementState::Vacant))
        .id()
}

fn stanced_ganger(app: &mut App, stance: StanceKind) -> Entity {
    GangerEntityBuilder::new()
        .stance(stance)
        .spawn(app.world_mut())
}

fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<EmplacementOccupant>(entity).map(|o| **o)
}

fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

fn toggle_and_settle(app: &mut App, request: SetEmplacement) {
    app.world_mut().write_message(request);
    app.update();
    app.update();
}

#[test]
fn occupy_forces_high_band_and_vacate_restores_from_stance() {
    let at = key(4, 4, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger(&mut app, StanceKind::Crouching);
    app.update();

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "an emplacement spawns Vacant (the default)",
    );
    assert_eq!(
        occupant_band(&app, at),
        None,
        "an unmanned emplacement publishes no occupant band",
    );

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "occupy flips the emplacement to Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(ganger),
        "occupy records the manning ganger as the EmplacementOccupant",
    );
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::High),
        "occupy forces the occupant band to HIGH (it reads as HIGH cover)",
    );

    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "vacate flips the emplacement back to Vacant",
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "vacate removes the EmplacementOccupant record",
    );
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::Mid),
        "vacate restores the occupant band from its CROUCHING stance silhouette (MID, not the forced HIGH)",
    );
}

#[test]
fn vacate_restores_standing_band() {
    let at = key(7, 2, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::High),
        "a standing occupant's band restores to HIGH on vacate",
    );
}

#[test]
fn occupy_on_occupied_is_rejected_no_force_eject() {
    let at = key(5, 5, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let first = stanced_ganger(&mut app, StanceKind::Standing);
    let second = stanced_ganger(&mut app, StanceKind::Prone);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, first));
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, second));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "re-occupying an Occupied emplacement stays Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "re-occupying does NOT displace the seated occupant (no force-eject)",
    );
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::High),
        "the forced HIGH band is unchanged by the rejected re-occupy",
    );
}

#[test]
fn toggle_panic_free_on_non_emplacement() {
    let mut app = headless_app();
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(key(9, 9, 0))).id();
    let ganger = stanced_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(plain, ganger));
    assert!(
        state(&app, plain).is_none(),
        "a non-emplacement entity gains no EmplacementState from a stray SetEmplacement (panic-free)",
    );
}

#[test]
fn toggle_is_deterministic() {
    let at = key(3, 3, 0);
    let run = || {
        let mut app = headless_app();
        let emplacement = spawn_vacant_emplacement(&mut app, at);
        let ganger = stanced_ganger(&mut app, StanceKind::Standing);
        app.update();
        toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
        toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
        (state(&app, emplacement), occupant_band(&app, at))
    };
    assert_eq!(
        run(),
        run(),
        "identical spawn+toggle sequences yield identical end state (deterministic)",
    );
    assert_eq!(
        run(),
        (Some(EmplacementState::Vacant), Some(HeightBand::High)),
        "the settled end state is Vacant with the standing band restored",
    );
}

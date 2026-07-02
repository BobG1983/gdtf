//! GTW-543 — the emplacement enter/exit toggle drives the REAL
//! [`apply_emplacement_toggle`](super::apply_emplacement_toggle) (via
//! [`EmplacementTogglePlugin`](super::EmplacementTogglePlugin)) in a headless `App`.
//!
//! Mirrors the door-precedent test harness (`terrain/openable/test.rs`): `MinimalPlugins`
//! (no window/renderer), the grid resources, the
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) (which
//! OWNS the `SimSystems::Simulate` set the toggle `.in_set`s into) + the GTW-543
//! [`EmplacementTogglePlugin`], driving the REAL [`SetEmplacement`](super::SetEmplacement)
//! message + `app.update()`-ticking. The tests assert STRUCTURAL facts (state flip, occupant
//! record, occupant band), never brittle magnitudes; the restriction lints (`unwrap` /
//! `expect` / `panic`) are denied in tests too, so every read stays `Option`-shaped.

use bevy::prelude::{App, Entity, MinimalPlugins};

use super::{EmplacementOccupant, EmplacementState, EmplacementTogglePlugin, SetEmplacement};
use crate::{
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::TerrainCell,
};

/// A `(x, y, level)` cell-key helper.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build the headless app: `MinimalPlugins` + the grid resources + the occupancy-maintenance
/// plugin (owns the `SimSystems::Simulate` set) + the GTW-543 emplacement toggle plugin. The
/// `SurfaceGrid` is seeded because the maintenance plugin's `sync_destroyed_slab` reads it
/// `ResMut`.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(EmplacementTogglePlugin);
    app
}

/// Spawn a VACANT emplacement terrain entity at `at` — the enter/exit state components
/// `setup_battle` attaches for an `Emplacement` piece. Returns its `Entity`.
fn spawn_vacant_emplacement(app: &mut App, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((TerrainCell::new(at), EmplacementState::Vacant))
        .id()
}

/// Spawn a ganger-stand-in carrying `stance` (the occupant whose band the vacate path restores).
fn spawn_ganger(app: &mut App, stance: StanceKind) -> Entity {
    app.world_mut().spawn(Stance::new(stance)).id()
}

/// The emplacement's current [`EmplacementState`], if it still carries one.
fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

/// The emplacement's recorded occupant, if it carries an [`EmplacementOccupant`].
fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<EmplacementOccupant>(entity).map(|o| **o)
}

/// The grid's published occupant band at `at` — `None` if absent OR no band published.
fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

/// Send a [`SetEmplacement`] then settle (one tick applies the deferred `Commands`; a second
/// keeps the harness parallel to the door test's two-tick idiom, harmless for this toggle
/// which needs no downstream projection).
fn toggle_and_settle(app: &mut App, request: SetEmplacement) {
    app.world_mut().write_message(request);
    app.update();
    app.update();
}

/// Occupy flips `Vacant` → `Occupied`, records the occupant, and forces the occupant band to
/// HIGH at the emplacement cell (the "reads as HIGH cover" force). Vacate reverses all three —
/// state back to `Vacant`, the occupant record removed, and the band restored from the
/// occupant's stance silhouette.
#[test]
fn occupy_forces_high_band_and_vacate_restores_from_stance() {
    let at = key(4, 4, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    // A CROUCHING occupant — its stance silhouette is MID, so a correct restore lands on MID
    // (distinct from the forced HIGH, so the restore-from-stance is discriminating, not a
    // coincidental HIGH).
    let ganger = spawn_ganger(&mut app, StanceKind::Crouching);
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

    // OCCUPY — flip Occupied, record the occupant, force the band to HIGH.
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

    // VACATE — flip Vacant, drop the occupant, restore the band from the crouching stance (MID).
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

/// A standing occupant's restored band is HIGH (the standing silhouette) — the default-stance
/// path, distinct from the crouching test above.
#[test]
fn vacate_restores_standing_band() {
    let at = key(7, 2, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = spawn_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::High),
        "a standing occupant's band restores to HIGH on vacate",
    );
}

/// No-force-eject: sending OCCUPY for an ALREADY-occupied emplacement is a no-op — the
/// original occupant stays seated, the band stays forced HIGH, and the state stays Occupied.
/// (A second ganger cannot displace the first by re-occupying.)
#[test]
fn occupy_on_occupied_is_rejected_no_force_eject() {
    let at = key(5, 5, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let first = spawn_ganger(&mut app, StanceKind::Standing);
    let second = spawn_ganger(&mut app, StanceKind::Prone);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, first));
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );

    // A SECOND ganger tries to occupy the already-Occupied emplacement — the idempotent
    // same-state guard makes this a no-op (no force-eject).
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

/// A [`SetEmplacement`] targeting a plain (non-emplacement) or despawned entity is skipped
/// panic-free — it gains no [`EmplacementState`].
#[test]
fn toggle_panic_free_on_non_emplacement() {
    let mut app = headless_app();
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(key(9, 9, 0))).id();
    let ganger = spawn_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(plain, ganger));
    assert!(
        state(&app, plain).is_none(),
        "a non-emplacement entity gains no EmplacementState from a stray SetEmplacement (panic-free)",
    );
}

/// The toggle is DETERMINISTIC — the same spawn + occupy/vacate sequence yields the same end
/// state every run (two independent apps agree). No RNG, no frame-order dependence.
#[test]
fn toggle_is_deterministic() {
    let at = key(3, 3, 0);
    let run = || {
        let mut app = headless_app();
        let emplacement = spawn_vacant_emplacement(&mut app, at);
        let ganger = spawn_ganger(&mut app, StanceKind::Standing);
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

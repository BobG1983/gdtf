use bevy::prelude::{App, Entity, MinimalPlugins};

use super::*;
use crate::{
    ganger::{LifeState, Position},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
};

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build a headless app: `MinimalPlugins` (no window / renderer — C1), the
/// full 60×60×8 [`OccupancyGrid`] resource, and the maintenance plugin (which
/// registers the [`CoverDestroyed`] message + the three chained systems).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Read the grid resource out of the app world for assertions — `Option` so the
/// test never `unwrap`s (the restriction lints fire in tests too).
fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&at))
}

fn cover_destroyed(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_cover_destroyed(&at))
}

/// C9(a) — a ganger MOVES: one tick after mutating [`Position`], the OLD slot
/// is cleared and the NEW slot is marked, WITHOUT any full-grid-rebuild call.
///
/// Spawns a ganger at an initial cell, ticks once (initial placement marks the
/// start slot via the first-run `Changed` semantics), then mutates `Position`
/// to a new cell and ticks again. Asserts the start slot is now empty and the
/// new slot holds the entity — the in-place clear-old + mark-new of C3. The
/// grid is only ever maintained via the systems; `build_from_occupancy_input`
/// is never called.
#[test]
fn moved_ganger_clears_old_slot_and_marks_new() {
    let mut app = headless_app();
    let start = key(5, 6, 0);
    let dest = key(9, 2, 1);

    let ganger = app
        .world_mut()
        .spawn((Position::new(start), LifeState::Alive))
        .id();

    // First tick: initial placement (Position reads as Changed on first run).
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement must mark the start slot",
    );

    // Move it: mutate Position, then tick once.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot must be cleared after a move (C3)",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot must be marked after a move (C3)",
    );
}

/// C9(b) — a ganger DIES: flipping [`LifeState`] to [`LifeState::Dead`] and
/// ticking once clears its occupant slot.
///
/// Spawns + places a ganger, then flips its `LifeState` to `Dead` and ticks.
/// The slot it occupied must be freed (C4). In place — no rebuild.
#[test]
fn dead_ganger_clears_its_slot() {
    let mut app = headless_app();
    let at = key(12, 13, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the ganger must occupy its slot before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a dead ganger's occupant slot must be cleared (C4)",
    );
}

/// A DOWNED ganger frees its slot too — C4 covers Downed and Dead alike (only
/// non-Alive frees the cell).
#[test]
fn downed_ganger_clears_its_slot() {
    let mut app = headless_app();
    let at = key(20, 20, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a downed ganger's occupant slot must be cleared (C4)",
    );
}

/// A `LifeState` change that stays [`LifeState::Alive`] does NOT free the slot —
/// only going OUT (Downed / Dead) clears it (C4).
#[test]
fn still_alive_change_keeps_slot() {
    let mut app = headless_app();
    let at = key(7, 7, 1);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();

    // Touch LifeState (mark it changed) but leave it Alive.
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Alive;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "an Alive LifeState change must NOT free the slot (C4)",
    );
}

/// C9(c) — emitting a [`CoverDestroyed`] message and ticking once adds the cell
/// to the grid's destroyed-cover set.
///
/// Writes the message into the world buffer, ticks, and asserts the cell now
/// reads destroyed (and an unrelated cell does not). In place — no rebuild.
#[test]
fn cover_destroyed_message_marks_the_cell() {
    let mut app = headless_app();
    let smashed = key(30, 31, 3);
    let intact = key(0, 0, 0);

    app.world_mut().write_message(CoverDestroyed::new(smashed));
    app.update();

    assert_eq!(
        cover_destroyed(&app, smashed),
        Some(true),
        "a CoverDestroyed message must mark its cell destroyed (C5)",
    );
    assert_eq!(
        cover_destroyed(&app, intact),
        Some(false),
        "an unrelated cell must not be marked destroyed",
    );
}

/// Two consecutive moves keep the grid consistent — the second move clears the
/// FIRST destination (now the tracked previous slot), not the original start.
/// Proves the [`PrevSlot`] bookkeeping advances with each move.
#[test]
fn two_moves_track_the_previous_slot() {
    let mut app = headless_app();
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);
    let c = key(3, 3, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(a), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(b);
    }
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(c);
    }
    app.update();

    assert_eq!(grid_occupant(&app, a), None, "the original start is empty");
    assert_eq!(
        grid_occupant(&app, b),
        None,
        "the first destination is empty"
    );
    assert_eq!(
        grid_occupant(&app, c),
        Some(ganger),
        "only the latest destination holds the ganger",
    );
}

/// The [`PrevSlot`] newtype round-trips the slot it records — the bookkeeping
/// the move system relies on.
#[test]
fn prev_slot_round_trips() {
    let slot = key(4, 5, 6);
    assert_eq!(PrevSlot::new(slot).slot(), slot);
}

/// AC1 — [`SimSystems::Simulate`] is a public, hashable ordering set with the
/// required derives. Referencing the variant from the (in-crate, but
/// `pub`-reachable) test path and asserting equality/clone proves the variant is
/// public and that `Clone`/`Copy`/`PartialEq`/`Eq` are present; `cargo dbuild`
/// linking the binary confirms it is reachable downstream with no `unreachable_pub`.
#[test]
fn sim_systems_simulate_is_public_and_derives() {
    let set = SimSystems::Simulate;
    assert_eq!(
        set,
        SimSystems::Simulate,
        "the set compares equal to itself"
    );
    // `Copy` (a use after `set` was already read) and `Clone` both hold.
    assert_eq!(set, set.clone(), "the set clones to an equal value");
}

/// AC2/AC3 — after [`OccupancyMaintenancePlugin`] nests its chain under
/// [`SimSystems::Simulate`], the move-sync still fires through the set: a ganger
/// that moves clears its OLD slot and marks its NEW one within one `app.update()`,
/// proving set membership did not break execution (membership has no observable
/// beyond ordering + execution, so the behavioral assertion stands in for it).
#[test]
fn move_sync_fires_through_the_simulate_set() {
    let mut app = headless_app();
    let start = key(8, 8, 0);
    let dest = key(10, 12, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(start), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement marks the start slot through SimSystems::Simulate",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot is cleared running inside SimSystems::Simulate",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot is marked running inside SimSystems::Simulate",
    );
}

//! GTW-391 — dual-cell stair presence REGISTRATION rules: which occupants write
//! an upper-cell presence, and which stay lower-only.

use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
    metric::CellLevel,
    occupancy::OccupancyGrid,
};

/// GTW-391 Test 1: a Standing ganger on a stair tile registers BOTH the lower cell
/// (correct stance band) AND the upper cell (Low band) after the first tick.
#[test]
fn standing_stair_occupant_registers_upper_low_band() {
    let mut app = headless_app();
    let lower = key(10, 10, 1);
    let upper = key(10, 10, 2);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, lower),
        Some(ganger),
        "lower cell must be occupied",
    );
    assert_eq!(
        grid_band(&app, lower),
        Some(HeightBand::High),
        "lower cell must carry the stance band (Standing → High)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper cell must also be occupied (dual-cell stair presence, GTW-391)",
    );
    assert_eq!(
        grid_band(&app, upper),
        Some(HeightBand::Low),
        "upper cell carries the Low band (body protrusion into the storey above, GTW-391)",
    );
}

/// GTW-391 C2: a Crouching ganger on a stair tile registers BOTH the lower cell
/// (Crouching's Mid silhouette band) AND the upper cell (Low band) after the first
/// tick — the guard is `stance_kind != StanceKind::Prone`, so any non-prone stance
/// gets the upper presence, not only Standing.
#[test]
fn crouching_stair_occupant_registers_upper_low_band() {
    let mut app = headless_app();
    let lower = key(10, 10, 1);
    let upper = key(10, 10, 2);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Crouching),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, lower),
        Some(ganger),
        "lower cell must be occupied",
    );
    assert_eq!(
        grid_band(&app, lower),
        Some(HeightBand::Mid),
        "lower cell must carry the stance band (Crouching → Mid)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper cell must also be occupied for a Crouching stair occupant (non-prone, GTW-391 C2)",
    );
    assert_eq!(
        grid_band(&app, upper),
        Some(HeightBand::Low),
        "upper cell carries the Low band regardless of the non-prone stance (GTW-391 C2)",
    );
}

/// GTW-391 Test 4: a ground shooter's lower-cell path is unchanged — no upper
/// presence on a non-stair cell.
#[test]
fn non_stair_occupant_lower_only() {
    let mut app = headless_app();
    let at = key(5, 5, 0);
    // NOT marked as a stair — the upper cell must remain empty.
    let upper = key(5, 5, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, at), Some(ganger));
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "a non-stair occupant must not write an upper-cell presence",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 5: a Prone ganger on a stair tile has NO upper-cell presence.
#[test]
fn prone_stair_occupant_has_no_upper_presence() {
    let mut app = headless_app();
    let lower = key(12, 12, 0);
    let upper = key(12, 12, 1);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Prone),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, lower), Some(ganger));
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "a prone stair occupant must have no upper-cell presence (prone = lower-only)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 10: a stair tile at the top storey (`MAX_LEVELS - 1`) writes no
/// upper presence — `upper_cell` returns `None` at the ceiling.
#[test]
fn top_storey_stair_occupant_lower_only() {
    use crate::metric::MAX_LEVELS;
    let mut app = headless_app();
    let top = key(2, 2, MAX_LEVELS - 1); // last valid storey

    mark_stair(&mut app, top);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(top),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, top), Some(ganger));
    // No upper cell exists above the top storey — register_stair_presence returns None.
    // The out-of-range key is graceful (no panic, no write).
    let non_existent_upper = key(2, 2, MAX_LEVELS);
    assert_eq!(
        grid_occupant(&app, non_existent_upper),
        None,
        "no upper-cell presence at the top storey (out-of-range, no panic)",
    );
}

/// GTW-391 Test 12: when the upper cell is already occupied by a DIFFERENT entity,
/// the stair ganger gets lower-only registration — the occupancy guard (Blocker 3).
#[test]
fn stair_occupant_with_occupied_upper_cell_is_lower_only() {
    let mut app = headless_app();
    let stair = key(20, 20, 1);
    let upper = key(20, 20, 2);

    mark_stair(&mut app, stair);

    // Ganger B occupies the upper cell as its OWN lower cell (it stands on level 2).
    let ganger_b = app
        .world_mut()
        .spawn((
            Position::new(upper),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "ganger B must occupy the upper cell before A registers",
    );

    // Now spawn ganger A on the stair — its upper cell is blocked by B.
    let ganger_a = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger_a),
        "ganger A occupies its lower (stair) cell",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "ganger B's slot must NOT be stomped by A (occupancy guard, GTW-391 Test 12)",
    );

    // Move A off the stair — B's slot must remain intact.
    let dest = key(21, 21, 1);
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger_a) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "B's slot survives A's teardown (teardown never stomps another entity's slot)",
    );
}

/// GTW-391 Test 13: `build_from_occupancy_input` with a stair-cell set registers the
/// upper presence AT PLACEMENT (frame 0 — Blocker 2 resolved).
#[test]
fn initial_placement_on_stair_registers_upper() {
    use bevy::{ecs::world::World, platform::collections::HashSet};

    use crate::occupancy::{OccupancyInput, OccupantPlacement};

    let mut world = World::new();
    let entity = world.spawn_empty().id();

    let lower = key(4, 4, 2);
    let upper = key(4, 4, 3);
    let stair_cells: HashSet<CellLevel> = std::iter::once(lower).collect();

    let input = OccupancyInput {
        terrain:   Vec::new(),
        occupants: vec![OccupantPlacement::new(lower, entity, HeightBand::High)],
    };

    // Build the grid — this must register the upper presence synchronously.
    let grid = OccupancyGrid::build_from_occupancy_input(&input, &stair_cells);

    // Assert WITHOUT any app.update() — the upper presence is frame-0 (Blocker 2).
    assert_eq!(
        grid.occupant(&lower),
        Some(entity),
        "lower cell occupied at build",
    );
    assert_eq!(
        grid.occupant_band(&lower),
        Some(HeightBand::High),
        "lower band set at build",
    );
    assert_eq!(
        grid.occupant(&upper),
        Some(entity),
        "upper cell occupied at build — frame-0, no tick needed (GTW-391 Test 13)",
    );
    assert_eq!(
        grid.occupant_band(&upper),
        Some(HeightBand::Low),
        "upper cell carries Low band at build",
    );
}

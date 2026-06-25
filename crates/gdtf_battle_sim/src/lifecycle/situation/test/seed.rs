//! Resource-seed tests — the cover ledger, surface grid, occupancy grid, and
//! vertical-link graph each pour from the same fixture.

use super::support::*;

/// C8(d) — the `CoverLedger` is seeded from the SAME fixture: the wall cell
/// holds a full-HP, not-destroyed entry with the authored band + armor.
#[test]
fn cover_ledger_seeded_from_fixture_wall() {
    use crate::{
        armor::{ArmorHardness, ArmorProtection},
        cover::{CoverEntry, CoverHp, HeightBand},
    };
    let (situation, _alice, _bob, wall_cell, _slab) = minimal_fixture();
    let Some((app, _setup)) = run_setup(situation) else {
        return;
    };

    let ledger = app.world().get_resource::<CoverLedger>();
    assert!(ledger.is_some(), "setup must insert a CoverLedger resource");
    let Some(ledger) = ledger else {
        return;
    };
    // The wall entry is present and seeded from the test-wall registry piece
    // ("test-wall": hp=120, High, prot=8, hard=4 — the test registry values, not
    // shipped balance; asserting the seam carried the registry values through).
    let expected = CoverEntry::seeded(
        CoverHp::new(120),       // test-wall hp (test registry, not a balance pin)
        HeightBand::High,        // test-wall band
        ArmorProtection::new(8), // test-wall protection
        ArmorHardness::new(4),   // test-wall hardness
    );
    assert_eq!(
        ledger.peek(&wall_cell).copied(),
        Some(expected),
        "the ledger must hold the authored wall entry at the wall cell",
    );
    // It is full-HP and not destroyed (seeded shape).
    assert_eq!(
        ledger.peek(&wall_cell).map(|e| e.destroyed),
        Some(Destroyed::new(false)),
        "a freshly seeded wall is not destroyed",
    );
    // current_hp seeds to the authored max (the C4 seed invariant
    // current_hp == max_hp) — the GTW-154 precedent, not a magnitude pin.
    assert_eq!(
        ledger.peek(&wall_cell).map(|e| e.current_hp),
        ledger.peek(&wall_cell).map(|e| e.max_hp),
        "current_hp seeds to the authored max",
    );
}

/// C8(d) — the `SurfaceGrid` is seeded from the SAME fixture: the slab cell
/// reads Present, an un-authored cell reads Absent.
#[test]
fn surface_grid_seeded_from_fixture_slab() {
    let (situation, .., slab_cell) = minimal_fixture();
    let Some((app, _setup)) = run_setup(situation) else {
        return;
    };

    let surface = app.world().get_resource::<SurfaceGrid>();
    assert!(
        surface.is_some(),
        "setup must insert a SurfaceGrid resource"
    );
    let Some(surface) = surface else {
        return;
    };
    assert_eq!(
        surface.slab_state(&slab_cell),
        SlabState::Present,
        "the authored slab must read Present",
    );
    assert_eq!(
        surface.slab_state(&key(0, 0, 1)),
        SlabState::Absent,
        "an un-authored cell must read Absent",
    );
}

/// C8(d) — the `OccupancyGrid` is seeded from the SAME fixture: the wall cell
/// carries Wall terrain and blocks; each ganger cell carries its SPAWNED Entity
/// handle as the occupant (never a numeric id).
#[test]
fn occupancy_grid_seeded_from_fixture() {
    let (situation, alice_at, bob_at, wall_cell, _slab) = minimal_fixture();
    let Some((app, setup)) = run_setup(situation) else {
        return;
    };

    let grid = app.world().get_resource::<OccupancyGrid>();
    assert!(
        grid.is_some(),
        "setup must insert an OccupancyGrid resource"
    );
    let Some(grid) = grid else {
        return;
    };

    // Terrain slot from the wall.
    assert_eq!(
        grid.terrain(&wall_cell),
        TerrainKind::Wall,
        "the wall cell must carry Wall terrain",
    );
    assert!(grid.is_blocked(&wall_cell), "a standing wall blocks");

    // Occupant slots hold the SPAWNED Entity handles (matching the returned
    // placements), never a numeric id.
    let alice = setup.occupants[0].occupant;
    let bob = setup.occupants[1].occupant;
    assert_eq!(
        grid.occupant(&alice_at),
        Some(alice),
        "alice's cell holds alice's spawned Entity handle",
    );
    assert_eq!(
        grid.occupant(&bob_at),
        Some(bob),
        "bob's cell holds bob's spawned Entity handle",
    );
}

/// The `VerticalLinkGraph` is built + inserted from a valid authored link, and
/// is queryable from the world.
#[test]
fn setup_inserts_vertical_link_graph() {
    let lower = key(1, 2, 0);
    let upper = key(1, 2, 1);
    // Author both endpoint cells as slabs so the link does not dangle.
    let situation = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();

    let Some((app, _setup)) = run_setup(situation) else {
        return;
    };

    let graph = app.world().get_resource::<VerticalLinkGraph>();
    assert!(
        graph.is_some(),
        "setup must insert a VerticalLinkGraph resource",
    );
    let Some(graph) = graph else {
        return;
    };
    assert_eq!(graph.len(), 1, "the one valid authored link is indexed");
    assert_eq!(
        graph.links_from(&lower).count(),
        1,
        "the link departs the lower endpoint",
    );
}

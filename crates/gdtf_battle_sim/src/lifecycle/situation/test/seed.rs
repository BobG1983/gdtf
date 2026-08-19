use super::support::*;

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
    let expected = CoverEntry::seeded(
        CoverHp::new(120),
        HeightBand::High,
        ArmorProtection::new(8),
        ArmorHardness::new(4),
        TerrainPieceKind::Wall,
    );
    assert_eq!(
        ledger.peek(&wall_cell).copied(),
        Some(expected),
        "the ledger must hold the authored wall entry at the wall cell",
    );
    assert_eq!(
        ledger.peek(&wall_cell).map(|e| e.destroyed),
        Some(Destroyed::new(false)),
        "a freshly seeded wall is not destroyed",
    );
    assert_eq!(
        ledger.peek(&wall_cell).map(|e| e.current_hp),
        ledger.peek(&wall_cell).map(|e| e.max_hp),
        "current_hp seeds to the authored max",
    );
}

/// records each piece's own kind, not the kind its authoring list implies.
#[test]
fn cover_ledger_kind_comes_from_the_piece_not_the_authoring_list() {
    let (situation, cover_in_walls_cell, wall_in_scatter_cell) = crossed_terrain_fixture();
    let Some((app, _setup)) = run_setup(situation) else {
        return;
    };

    let ledger = app.world().get_resource::<CoverLedger>();
    assert!(ledger.is_some(), "setup must insert a CoverLedger resource");
    let Some(ledger) = ledger else {
        return;
    };

    let cover_kind = ledger.peek(&cover_in_walls_cell).map(|entry| entry.kind);
    assert_eq!(
        cover_kind,
        Some(TerrainPieceKind::Cover),
        "a Cover-spec piece authored in the walls list must seed a Cover ledger entry, \
         found {cover_kind:?}",
    );

    let wall_kind = ledger.peek(&wall_in_scatter_cell).map(|entry| entry.kind);
    assert_eq!(
        wall_kind,
        Some(TerrainPieceKind::Wall),
        "a Wall-spec piece authored in the scatter list must seed a Wall ledger entry, \
         found {wall_kind:?}",
    );
}

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

    assert_eq!(
        grid.terrain(&wall_cell),
        TerrainKind::Wall,
        "the wall cell must carry Wall terrain",
    );
    assert!(*grid.is_blocked(&wall_cell), "a standing wall blocks");

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

#[test]
fn setup_inserts_vertical_link_graph() {
    let lower = key(1, 2, 0);
    let upper = key(1, 2, 1);
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

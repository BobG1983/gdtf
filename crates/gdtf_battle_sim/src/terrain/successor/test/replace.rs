use super::support::*;
use crate::{
    occupancy::{OccupancyGrid, TerrainKind},
    occupancy_sync::TerrainPieceDestroyed,
    surface::{SlabState, SurfaceGrid},
    terrain::entity::{TerrainIndex, TerrainIndexKey},
};

#[test]
fn a_destroyed_piece_is_replaced_by_the_def_its_leaves_behind_names() {
    let at = key(6, 6, 0);
    let mut app = battle_with(&[(at, SMASHED_COVER)]);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    assert_eq!(
        pieces_keyed(&mut app, at, SUCCESSOR_WALL).len(),
        1,
        "the cell holds one piece carrying the successor def's own key",
    );
    assert!(
        pieces_keyed(&mut app, at, SMASHED_COVER).is_empty(),
        "the destroyed piece is despawned in the same update it was marked",
    );
}

#[test]
fn a_piece_that_leaves_nothing_behind_clears_its_own_kind() {
    let at = key(7, 6, 0);
    let mut app = battle_with(&[(at, PLAIN_COVER)]);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    let index = app.world().get_resource::<TerrainIndex>();
    assert_eq!(
        index.and_then(|index| index.get(&TerrainIndexKey::Cover(at))),
        None,
        "the cover key is dropped, and the slab key at the same cell is not read here",
    );
    let grid = app.world().get_resource::<OccupancyGrid>();
    assert_eq!(
        grid.map(|grid| grid.terrain(&at)),
        Some(TerrainKind::Open),
        "the occupancy grid reads Open at the cell the cover stood on",
    );
    assert!(
        pieces_keyed(&mut app, at, PLAIN_COVER).is_empty(),
        "no entity carries both that cell and the destroyed def's key",
    );
}

#[test]
fn two_messages_for_one_cell_in_one_update_leave_one_successor() {
    let at = key(8, 6, 0);
    let mut app = battle_with(&[(at, SMASHED_COVER)]);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    let standing = pieces_keyed(&mut app, at, SUCCESSOR_WALL);
    assert_eq!(
        standing.len(),
        1,
        "the second message names a key this run already acted on, so it spawns nothing more",
    );
    let index = app.world().get_resource::<TerrainIndex>();
    assert_eq!(
        index.and_then(|index| index.get(&TerrainIndexKey::Cover(at))),
        standing.first().copied(),
        "the index points at the successor rather than having been cleared by the second \
         message",
    );
}

#[test]
fn a_destroyed_cover_leaves_the_slab_under_it_standing() {
    let at = key(9, 6, 0);
    let mut app = battle_with(&[(at, PLAIN_COVER)]);
    assert_eq!(
        app.world()
            .get_resource::<SurfaceGrid>()
            .map(|surface| surface.slab_state(&at)),
        Some(SlabState::Present),
        "the case needs a slab standing under the cover before the smash",
    );

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    assert_eq!(
        app.world()
            .get_resource::<SurfaceGrid>()
            .map(|surface| surface.slab_state(&at)),
        Some(SlabState::Present),
        "a Cover message clears only the occupancy grid, so the floor under it still stands",
    );
}

#[test]
fn a_successor_slab_over_a_stair_endpoint_is_braced_the_way_the_seeded_slab_was() {
    let below = key(10, 6, 0);
    let at = key(10, 6, 1);
    let mut app = battle_with_braced_slab(below, at);
    assert!(
        carries_brace(&mut app, at, BRACED_SLAB),
        "the case needs the authored slab braced by the stair below it before the smash",
    );

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, SLAB_KIND));
    app.update();

    assert!(
        carries_brace(&mut app, at, SUCCESSOR_SLAB),
        "the successor carries every component seed_slab_terrain attaches, TerrainBrace \
         included",
    );
}

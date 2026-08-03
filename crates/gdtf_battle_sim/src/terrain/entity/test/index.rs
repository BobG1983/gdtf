use super::support::*;
use crate::{
    battle::SetupBattleRequested,
    cover::{CoverHp, CoverLedger},
    terrain::entity::{TerrainCell, TerrainIndex, TerrainIndexKey, TerrainPieceKind},
    test_support::{SituationBuilder, ganger_at},
};


#[test]
fn test3_queryable_by_cell() {
    let wall_cell = cl(2, 2, 0);
    let slab_cell = cl(3, 4, 1);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x02),
    ));
    app.update();

    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 3: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };

    let cover_entity_opt = index.get(&TerrainIndexKey::Cover(wall_cell));
    assert!(
        cover_entity_opt.is_some(),
        "Test 3 (C4): wall must be queryable by cell"
    );
    let Some(cover_entity) = cover_entity_opt else {
        return;
    };
    let cover_cell_comp = world.get::<TerrainCell>(cover_entity).copied();
    let cover_kind = world.get::<TerrainPieceKind>(cover_entity).copied();
    assert_eq!(
        cover_cell_comp.as_deref().copied(),
        Some(wall_cell),
        "Test 3: TerrainCell must equal the authored wall cell",
    );
    assert_eq!(
        cover_kind,
        Some(TerrainPieceKind::Wall),
        "Test 3: wall entity kind must be Wall",
    );

    let slab_entity_opt = index.get(&TerrainIndexKey::Slab(slab_cell));
    assert!(
        slab_entity_opt.is_some(),
        "Test 3 (C4): slab must be queryable by cell"
    );
    let Some(slab_entity) = slab_entity_opt else {
        return;
    };
    let slab_cell_comp = world.get::<TerrainCell>(slab_entity).copied();
    let slab_kind = world.get::<TerrainPieceKind>(slab_entity).copied();
    assert_eq!(
        slab_cell_comp.as_deref().copied(),
        Some(slab_cell),
        "Test 3: TerrainCell must equal the authored slab cell",
    );
    assert_eq!(
        slab_kind,
        Some(TerrainPieceKind::Slab),
        "Test 3: slab entity kind must be Slab",
    );
}


#[test]
fn test6_bridge_max_hp_on_entity_live_hp_in_ledger() {
    let wall_cell = cl(2, 2, 0);
    let authored_max = CoverHp::new(120);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x04),
    ));
    app.update();

    let world = app.world_mut();

    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 6: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };
    let entity_opt = index.get(&TerrainIndexKey::Cover(wall_cell));
    assert!(entity_opt.is_some(), "Test 6: wall must be in TerrainIndex");
    let Some(entity) = entity_opt else {
        return;
    };

    let entity_max = world.get::<CoverHp>(entity).copied();
    assert_eq!(
        entity_max,
        Some(authored_max),
        "Test 6 (C3): entity CoverHp must equal authored max",
    );

    let ledger_opt = world.get_resource::<CoverLedger>();
    assert!(ledger_opt.is_some(), "Test 6: CoverLedger must be present");
    let Some(ledger) = ledger_opt else {
        return;
    };
    let ledger_entry = ledger.peek(&wall_cell);
    assert!(
        ledger_entry.is_some(),
        "Test 6: CoverLedger must have an entry for the authored wall cell after setup",
    );
    let live_hp = ledger_entry.map(|e| e.current_hp);
    assert_eq!(
        live_hp,
        Some(authored_max),
        "Test 6 (bridge): live HP from CoverLedger must equal max at setup (no hits yet)",
    );
}


#[test]
fn test7_typed_key_no_collision_at_shared_cell() {
    let shared_cell = cl(5, 5, 0);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(shared_cell)
        .slab_at(shared_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x05),
    ));
    app.update();

    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 7: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };

    assert!(
        index.len() >= 2,
        "Test 7 (major 4): TerrainIndex must have at least 2 entries for a shared-cell \
         wall + slab (Cover(cell) and Slab(cell) are distinct keys)",
    );

    let cover_entity = index.get(&TerrainIndexKey::Cover(shared_cell));
    let slab_entity = index.get(&TerrainIndexKey::Slab(shared_cell));
    assert!(
        cover_entity.is_some(),
        "Test 7: Cover(shared_cell) must resolve to an entity",
    );
    assert!(
        slab_entity.is_some(),
        "Test 7: Slab(shared_cell) must resolve to an entity",
    );

    assert_ne!(
        cover_entity, slab_entity,
        "Test 7: the cover and slab entities at the shared cell must be distinct",
    );
}

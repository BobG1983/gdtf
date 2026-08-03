use super::support::*;
use crate::{
    battle::{SetupBattleRequested, TeardownBattleRequested},
    slab::SlabLedger,
    terrain::entity::TerrainIndex,
    test_support::{SituationBuilder, ganger_at},
};


#[test]
fn test5_terrain_index_and_slab_ledger_removed_on_teardown() {
    let mut app = headless_app();

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(cl(2, 2, 0))
        .slab_at(cl(3, 3, 1))
        .build();

    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x03),
    ));
    app.update();
    drain_ready(&mut app);

    assert!(
        app.world().get_resource::<TerrainIndex>().is_some(),
        "Test 5 precondition: TerrainIndex must be present after setup",
    );
    assert!(
        app.world().get_resource::<SlabLedger>().is_some(),
        "Test 5 precondition: SlabLedger must be present after setup",
    );

    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    assert!(
        app.world().get_resource::<TerrainIndex>().is_none(),
        "Test 5 (blocker 1): TerrainIndex must be absent after teardown",
    );
    assert!(
        app.world().get_resource::<SlabLedger>().is_none(),
        "Test 5 (blocker 1, leak fix): SlabLedger must be absent after teardown (pre-existing \
         leak — it was never removed until GTW-395)",
    );
}

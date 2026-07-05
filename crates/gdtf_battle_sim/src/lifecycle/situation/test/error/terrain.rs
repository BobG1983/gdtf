//! GTW-491 — the missing terrain-def UUID abort.

use super::super::support::*;

/// GTW-491 (C1, pin-discriminating) — a `scatter` entry referencing a terrain definition
/// UUID that is ABSENT from the [`TerrainDefRegistry`] makes `setup_battle` return
/// `Err(BattleSetupError::TerrainNotFound)` (now keyed by the unresolved [`TerrainUuid`])
/// and spawn NOTHING (terrain resolution runs before the spawn loop) — the abort-first
/// contract, the terrain mirror of `setup_errors_on_a_missing_weapon_key`. A wrong/missing
/// UUID yielding anything but `TerrainNotFound` reddens this (the C1 pin).
#[test]
fn setup_errors_on_a_missing_terrain_key() {
    // A ganger with the default (valid) weapon + armor keys, so the weapon + armor
    // resolution passes and the GTW-491 terrain resolution is reached. The scatter piece
    // names a def UUID that the test terrain registry does NOT hold.
    let missing = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_dead_0000_0001));
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .with_scatter(CoverSpawn::new(key(2, 2, 0), missing))
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    // The registry DOES exist (so we reach key resolution) but lacks the authored key.
    let terrain = test_terrain_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::TerrainNotFound { piece: missing }),
        "a missing terrain key must abort setup with TerrainNotFound (no panic)",
    );

    // Nothing was spawned, and no terrain ledger was inserted (terrain resolution
    // aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a terrain-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a terrain-resolution abort must insert no resources",
    );
    assert!(
        world.get_resource::<FloorCostGrid>().is_none(),
        "a terrain-resolution abort must insert no FloorCostGrid",
    );
}

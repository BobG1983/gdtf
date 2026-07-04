//! GTW-414 / GTW-415 — the REAL-ASSET live-battle-start proof (the gangs
//! family's bespoke migration pin, split from `load_gangs.rs` under the repo
//! file caps by GTW-580 — kept verbatim, never genericized).
//!
//! This is the feature-completeness proof the seeded-fixture regression test
//! (`gdtf_battle_sim`'s `shipped_situation_ron_drives_the_real_setup_path`, which
//! synthesizes its rosters via `SituationBuilder::build_with_gangs` /
//! `test_gang_registry`) cannot give: it drives the WHOLE real Load flow — the genuine
//! `AssetServer` rooted at the workspace `assets/` loads `assets/content/gangs/*.gang.ron`
//! into a `GangRegistry`, WITHOUT seeding any default registry — and then proves the
//! REAL shipped `skirmish.ron` + that real `GangRegistry` (+ the real weapon / armor /
//! terrain registries + stat tuning, also resolved by the Load flow) spawn the CONCRETE
//! expected ganger set through the authoritative `setup_battle`.
//!
//! It would FAIL closed if the GTW-415 loader were missing (the registry stays empty,
//! every placed ganger's `(gang, member)` ref fails to resolve → `GangNotFound` → no
//! gangers spawn) — so it is the live-battle-start-is-whole proof. Asserting the
//! migrated values match the pre-migration inline list is the EXPLICIT exception to the
//! "loaders must not pin shipped magnitudes" rule — it proves the migration preserved
//! them verbatim. The expected set + its field-for-field assertions live in
//! [`expected`].

/// The expected shipped spawn set + its field-for-field assertions (split
/// under the repo file caps). `#[path]` because a test-crate ROOT resolves a
/// bare `mod` beside itself in `tests/`, not in a same-named subdirectory.
#[path = "load_gangs_spawn/expected.rs"]
mod expected;

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
    scene::ScenePlugin,
};
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    ArmorRegistry, BattleRegistries, GangRegistry, GangerName, TerrainDefRegistry, WeaponRegistry,
    setup_battle,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async
/// asset load resolving — a safety net against a genuine never-resolve hang, not a timing
/// budget (the GTW-305 / `load_weapons` precedent).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Part (b) — the REAL shipped `skirmish.ron` + the real `GangRegistry` (both resolved by
/// the genuine Load flow) drive the authoritative `setup_battle` to spawn the CONCRETE
/// expected ganger set: same `GangerName`s (Alex Mercer / Vex 1 / Vex 2), same eight
/// attribute values, same weapon, same placement (position / facing / stance / aiming /
/// life state) and faction as the pre-migration inline list.
///
/// This is the live-battle-start-is-whole proof: it FAILS closed if the gang refs fail to
/// resolve (no gangers spawn). Asserting the migrated values match the pre-migration ones
/// is the deliberate exception to the no-pinning rule — it proves the GTW-414/415
/// migration preserved them verbatim.
///
/// PIN: it fails if the loader is missing (empty registry → `GangNotFound` → zero
/// gangers), if a roster value drifted from the pre-migration inline list, or if the
/// placement / faction were lost in the split.
#[test]
fn real_skirmish_with_real_gangs_spawns_the_expected_set() {
    // 1. Drive the real Load flow until every resource setup_battle needs is resolved
    //    from the real assets (NO seeded defaults — these come from disk).
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<GangRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<ArmorRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<GangerStatTuning>(&mut app, LOAD_SAFETY_NET);
    // GTW-545: the FieldDefRegistry catalog too — the shipped skirmish.ron now authors a
    // `fields:` toxic-pool placement, so setup_battle must resolve its key against the loaded
    // catalog (else FieldNotFound).
    advance_until_resource_exists::<gdtf_battle_sim::FieldDefRegistry>(&mut app, LOAD_SAFETY_NET);

    // 2. Read the resolved-from-disk resources out of the loaded world (all Clone), so a
    //    FRESH headless app can run setup_battle against the REAL data. Every one must be
    //    present (the gate waited for them) — assert that, then unpack panic-free.
    let world = app.world();
    let situation = world
        .get_resource::<LoadedSituation>()
        .map(|s| (**s).clone());
    let gangs = world.get_resource::<GangRegistry>().cloned();
    let weapons = world.get_resource::<WeaponRegistry>().cloned();
    let armor = world.get_resource::<ArmorRegistry>().cloned();
    let terrain = world.get_resource::<TerrainDefRegistry>().cloned();
    let stat_tuning = world.get_resource::<GangerStatTuning>().cloned();
    let field_defs = world
        .get_resource::<gdtf_battle_sim::FieldDefRegistry>()
        .cloned();
    let all_present = situation.is_some()
        && gangs.is_some()
        && weapons.is_some()
        && armor.is_some()
        && terrain.is_some()
        && stat_tuning.is_some()
        && field_defs.is_some();
    assert!(
        all_present,
        "every real Load resource setup_battle needs must be present",
    );
    let (
        Some(situation),
        Some(gangs),
        Some(weapons),
        Some(armor),
        Some(terrain),
        Some(stat_tuning),
        Some(field_defs),
    ) = (
        situation,
        gangs,
        weapons,
        armor,
        terrain,
        stat_tuning,
        field_defs,
    )
    else {
        return;
    };

    // 3. Run the authoritative setup_battle on a fresh MinimalPlugins+Scene app against the
    //    REAL skirmish + REAL gang/weapon/armor/terrain registries, then drive SpawnScene.
    let Some(mut battle) = run_real_setup(
        situation,
        &gangs,
        weapons,
        armor,
        terrain,
        stat_tuning,
        &field_defs,
    ) else {
        return;
    };

    // 4. Build a name → spawned-entity map in one query, then assert each expected ganger
    //    spawned with its concrete migrated values (placement + attributes + weapon), and
    //    the migrated weapon + armor KEYS (data-level, via the real gang registry).
    let spawned = ganger_entities(battle.world_mut());
    expected::assert_expected_set(battle.world(), &spawned, &gangs);
}

/// Run the authoritative `setup_battle` on a fresh `MinimalPlugins` + `AssetPlugin` +
/// `ScenePlugin` app against the REAL skirmish + registries, driving the `SpawnScene`
/// schedule so the deferred `bsn!` ganger components materialize. Returns the live app, or
/// `None` (asserting first) if setup did not succeed.
fn run_real_setup(
    situation: Situation,
    gangs: &GangRegistry,
    weapons: WeaponRegistry,
    armor: ArmorRegistry,
    terrain: TerrainDefRegistry,
    stat_tuning: GangerStatTuning,
    field_defs: &gdtf_battle_sim::FieldDefRegistry,
) -> Option<App> {
    let fallback_floor_cost = CombatTuning::default().move_costs.open;
    let mut battle = App::new();
    battle.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let gangs = gangs.clone();
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup (these gangers author none → `fists`).
    let melee = gdtf_battle_sim::test_support::test_melee_weapon_registry();
    // GTW-545: the loaded field catalog so the shipped skirmish's authored toxic-pool field
    // resolves its key (else FieldNotFound).
    let field_defs = field_defs.clone();
    let outcome = battle
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &weapons,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                )
                .with_field_defs(&field_defs),
                fallback_floor_cost,
                &mut commands,
            )
        });
    let succeeded = matches!(outcome, Ok(Ok(_)));
    assert!(
        succeeded,
        "setup_battle must succeed on the REAL skirmish + REAL gangs (a missing gang ref \
         would abort with GangNotFound); outcome was {outcome:?}",
    );
    if !succeeded {
        return None;
    }
    // Drive the SpawnScene schedule so the deferred `bsn!` ganger components materialize.
    battle.update();
    Some(battle)
}

/// Collect every spawned ganger entity keyed by its [`GangerName`] string — one query so
/// the per-ganger assertions read from a `&World` (no per-lookup `&mut World`).
fn ganger_entities(world: &mut World) -> std::collections::HashMap<String, Entity> {
    let mut query = world.query::<(Entity, &GangerName)>();
    query
        .iter(world)
        .map(|(entity, name)| ((**name).clone(), entity))
        .collect()
}

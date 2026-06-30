//! GTW-414 / GTW-415 — the REAL-ASSET gang-loader + live-battle-start proof.
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
//! gangers spawn) — so it is the live-battle-start-is-whole proof.
//!
//! Two tests:
//!
//! - [`real_asset_resolves_gang_registry_keyed_by_filename`] — part (a): the REAL gangs
//!   folder loads into a NON-EMPTY `GangRegistry` holding `gang_0` + `gang_1` and their
//!   members, via the real Load flow, with NO seeded default registry.
//! - [`real_skirmish_with_real_gangs_spawns_the_expected_set`] — part (b): the REAL
//!   shipped `skirmish.ron` + the real `GangRegistry` drive `setup_battle` to spawn the
//!   concrete pre-migration ganger set (same names, same eight attribute values, same
//!   weapon + armor, same placement + faction + life state). This asserting the migrated
//!   values match the pre-migration inline list is the EXPLICIT exception to the
//!   "loaders must not pin shipped magnitudes" rule — it proves the migration preserved
//!   them verbatim.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
    scene::ScenePlugin,
};
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    Aim, Aiming, ArmorRegistry, BattleRegistries, Cell, CellLevel, Cool, Direction, Facing,
    Faction, GangName, GangRegistry, GangerName, Grit, Level, LifeState, Luck, Position, Reflexes,
    Speed, Stance, StanceKind, Strength, TerrainDefRegistry, Toughness, WeaponName, WeaponRegistry,
    Wields, setup_battle,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async
/// asset load resolving — a safety net against a genuine never-resolve hang, not a timing
/// budget (the GTW-305 / `load_weapons` precedent).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Part (a) — with a real `AssetServer` rooted at the workspace `assets/`, entering
/// `Load` loads `assets/content/gangs/*.gang.ron` and builds a `GangRegistry` keyed by
/// each gang's filename stem (minus the `.gang` infix). Proves the REAL gangs folder
/// loaded into the registry keyed by filename (`gang_0` / `gang_1` resolve) with their
/// rosters — driven WITHOUT seeding any default registry, so existence implies the real
/// resolve ran.
///
/// PIN: this fails if the GTW-415 loader is missing / mis-wired (no `GangRegistry` is
/// ever inserted, so `advance_until_resource_exists` times out and the registry stays
/// `None`), or if the keying drops the `gang_0` / `gang_1` entries.
#[test]
fn real_asset_resolves_gang_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async gangs folder load: wait until the GangRegistry is inserted,
    // not a fixed frame count. Cap is a safety net (GTW-305). NO default registry is
    // seeded, so this only resolves via the real folder load.
    advance_until_resource_exists::<GangRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<GangRegistry>();
    assert!(
        registry.is_some(),
        "the real gangs folder load must insert a GangRegistry within the safety-net budget",
    );
    let Some(registry) = registry else {
        return;
    };

    assert!(
        !registry.is_empty(),
        "the resolved GangRegistry must carry the authored (non-empty) gangs",
    );

    // gang_0 resolves and holds "Alex Mercer".
    let gang_0 = registry.roster(&GangName::new("gang_0".to_owned()));
    assert!(
        gang_0
            .and_then(|roster| roster.member(&GangerName::new("Alex Mercer".to_owned())))
            .is_some(),
        "the registry must hold gang_0 (keyed by gang_0.gang.ron's stem) with member \"Alex Mercer\"",
    );

    // gang_1 resolves and holds both "Vex 1" and "Vex 2".
    let gang_1 = registry.roster(&GangName::new("gang_1".to_owned()));
    assert!(
        gang_1
            .and_then(|roster| roster.member(&GangerName::new("Vex 1".to_owned())))
            .is_some(),
        "the registry must hold gang_1 (keyed by gang_1.gang.ron's stem) with member \"Vex 1\"",
    );
    assert!(
        gang_1
            .and_then(|roster| roster.member(&GangerName::new("Vex 2".to_owned())))
            .is_some(),
        "the registry's gang_1 must also hold member \"Vex 2\"",
    );
}

/// The concrete, expected spawn record for one ganger — the pre-GTW-414 inline value an
/// authored ganger carried, now sourced from the migrated gang roster + situation
/// placement. Compared field-for-field against the spawned entity so the migration's
/// value-preservation is proven exactly (the explicit pin-the-migrated-values exception).
struct Expected {
    name:       &'static str,
    /// The eight direct attributes (`speed, aim, strength, toughness, reflexes, cool,
    /// grit, luck`) — the raw authored potential, verbatim from the gang `.ron`.
    attributes: [f32; 8],
    weapon:     &'static str,
    armor:      &'static str,
    /// The gang the situation references this member through (`gang_0` / `gang_1`) — the
    /// key the data-level weapon + armor assertions resolve the roster member against.
    gang:       &'static str,
    position:   CellLevel,
    faction:    u8,
    facing:     Direction,
    stance:     StanceKind,
    aiming:     bool,
    life_state: LifeState,
}

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
    let all_present = situation.is_some()
        && gangs.is_some()
        && weapons.is_some()
        && armor.is_some()
        && terrain.is_some()
        && stat_tuning.is_some();
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
    ) = (situation, gangs, weapons, armor, terrain, stat_tuning)
    else {
        return;
    };

    // 3. Run the authoritative setup_battle on a fresh MinimalPlugins+Scene app against the
    //    REAL skirmish + REAL gang/weapon/armor/terrain registries, then drive SpawnScene.
    let expected = expected_set();
    let Some(mut battle) = run_real_setup(situation, &gangs, weapons, armor, terrain, stat_tuning)
    else {
        return;
    };

    // 4. Build a name → spawned-entity map in one query, then assert each expected ganger
    //    spawned with its concrete migrated values (placement + attributes + weapon), and
    //    the migrated weapon + armor KEYS (data-level, via the real gang registry).
    let spawned = ganger_entities(battle.world_mut());
    assert_eq!(
        spawned.len(),
        expected.len(),
        "setup_battle must spawn exactly the {} expected gangers from the real skirmish + gangs",
        expected.len(),
    );
    let battle_world = battle.world();
    for want in &expected {
        assert!(
            spawned.contains_key(want.name),
            "expected ganger {:?} must have spawned",
            want.name,
        );
        let Some(&entity) = spawned.get(want.name) else {
            continue;
        };
        assert_ganger(battle_world, entity, want);
        assert_roster_keys(&gangs, want);
    }
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
) -> Option<App> {
    let fallback_floor_cost = CombatTuning::default().move_costs.open;
    let mut battle = App::new();
    battle.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let gangs = gangs.clone();
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup (these gangers author none → `fists`).
    let melee = gdtf_battle_sim::test_support::test_melee_weapon_registry();
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
                ),
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

/// The expected pre-migration ganger set (the inline list `skirmish.ron` used to carry,
/// now sourced from the migrated gang rosters + situation placements).
fn expected_set() -> [Expected; 3] {
    [
        Expected {
            name:       "Alex Mercer",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            weapon:     "stub_pistol",
            armor:      "flak_vest",
            gang:       "gang_0",
            position:   cell(5, 6, 0),
            faction:    0,
            facing:     Direction::East,
            stance:     StanceKind::Standing,
            aiming:     false,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 1",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            position:   cell(12, 9, 0),
            faction:    1,
            facing:     Direction::West,
            stance:     StanceKind::Crouching,
            aiming:     true,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 2",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            position:   cell(12, 12, 0),
            faction:    1,
            facing:     Direction::West,
            stance:     StanceKind::Crouching,
            aiming:     true,
            life_state: LifeState::Alive,
        },
    ]
}

/// A `(cell, level)` key for an expected position.
fn cell(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
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

/// Assert the spawned ganger entity carries the concrete migrated placement + attribute
/// + weapon values from `want` (field-for-field — the migration-preserved-them proof).
fn assert_ganger(world: &World, entity: Entity, want: &Expected) {
    // Placement is components on the ganger entity.
    assert_eq!(
        world.get::<Position>(entity).map(|p| **p),
        Some(want.position),
        "{}: spawned Position must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Faction>(entity).map(|f| **f),
        Some(want.faction),
        "{}: spawned Faction must match the situation-assigned side",
        want.name,
    );
    assert_eq!(
        world.get::<Facing>(entity).map(|f| **f),
        Some(want.facing),
        "{}: spawned Facing must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Stance>(entity).map(|s| **s),
        Some(want.stance),
        "{}: spawned Stance must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Aiming>(entity).map(|a| **a),
        Some(want.aiming),
        "{}: spawned Aiming must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<LifeState>(entity).copied(),
        Some(want.life_state),
        "{}: spawned LifeState must match the migrated placement",
        want.name,
    );

    // The eight direct attributes (the raw authored potential, verbatim from the roster).
    let attributes = [
        world.get::<Speed>(entity).map(|v| **v),
        world.get::<Aim>(entity).map(|v| **v),
        world.get::<Strength>(entity).map(|v| **v),
        world.get::<Toughness>(entity).map(|v| **v),
        world.get::<Reflexes>(entity).map(|v| **v),
        world.get::<Cool>(entity).map(|v| **v),
        world.get::<Grit>(entity).map(|v| **v),
        world.get::<Luck>(entity).map(|v| **v),
    ];
    for (i, expected) in want.attributes.iter().enumerate() {
        assert_eq!(
            attributes[i],
            Some(*expected),
            "{}: attribute[{i}] must match the migrated roster value {expected}",
            want.name,
        );
    }

    // The wielded weapon: the ganger relates to one weapon entity (ganger → Wields → the
    // weapon entity), whose WeaponName is the resolved key from the migrated roster — so
    // the ganger ends up ARMED from the migrated roster weapon, spawned through the real
    // setup path.
    let weapon_name = world
        .get::<Wields>(entity)
        .and_then(Wields::weapon)
        .and_then(|weapon_entity| world.get::<WeaponName>(weapon_entity))
        .map(|n| (**n).clone());
    assert_eq!(
        weapon_name,
        Some(want.weapon.to_owned()),
        "{}: the wielded weapon entity's WeaponName must be the migrated roster weapon key",
        want.name,
    );
}

/// Data-level weapon + armor key proof: the migrated roster member's weapon + armor keys
/// (resolved through the real gang registry) match the pre-migration inline values. The
/// keys are not spawned components, so they are asserted against the registry the placement
/// resolves through.
fn assert_roster_keys(gangs: &GangRegistry, want: &Expected) {
    let member = gangs
        .roster(&GangName::new(want.gang.to_owned()))
        .and_then(|roster| roster.member(&GangerName::new(want.name.to_owned())));
    assert!(
        member.is_some(),
        "{}: must resolve in the real gang registry",
        want.name,
    );
    let Some(member) = member else {
        return;
    };
    assert_eq!(
        (*member.weapon).as_str(),
        want.weapon,
        "{}: migrated roster weapon key must match the pre-migration value",
        want.name,
    );
    assert_eq!(
        (*member.armor).as_str(),
        want.armor,
        "{}: migrated roster armor key must match the pre-migration value",
        want.name,
    );
}

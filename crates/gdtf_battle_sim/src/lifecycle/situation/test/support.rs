//! Shared test fixtures + helpers for the `situation` module tests — the
//! `MinimalPlugins` setup drivers, the C8 minimal fixture, and the shipped-asset
//! `include_str!` constants. The reusable ganger / situation / registry builders
//! now live in the crate-central [`crate::test_support`]; this file re-exports them
//! and keeps only the `situation`-module-specific harness. Each concern file does
//! `use super::support::*;` to reach them.

pub(super) use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
    scene::ScenePlugin,
};

pub(super) use super::super::*;
// The canonical shared builders + specs + registries + the `(cell, level)` key
// helper (consolidated out of this file's former local copies).
pub(super) use crate::test_support::{
    GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, arbitrary_armor, ganger_at, key,
    test_armor_registry, test_gang_registry, test_melee_weapon_registry, test_terrain_registry,
    test_weapon_registry as test_registry,
};
pub(super) use crate::{
    armor::{ArmorHardness, ArmorName, ArmorProtection, ArmorRegistry, ArmorSpec, BodyPart, Wears},
    cover::{CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{
        Aiming, Facing, Faction, GangName, GangRegistry, GangRoster, GangerAttributes, GangerName,
        Hp, HpMax, LifeState, Luck, Position, Shooting, Stance, Toughness, Tu, TuMax, Wounds,
        WoundsMax, derive_stats,
    },
    inflicted_wound::InflictedWounds,
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    situation::CoverSpawn,
    surface::{SlabState, SurfaceGrid},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainUuid,
        },
        entity::{BlocksPathfinding, TerrainCell, TerrainPieceKind},
        floor::FloorCostGrid,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    tuning::GangerStatTuning,
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireMode, Weapon, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy, Wields},
};

/// The two shipped weapon `.ron` files, read at compile time via the same
/// `include_str!` pattern the shipped situation uses — the REAL on-disk authored
/// weapons (keyed by their filename stems), so an AC5 regression in either file
/// turns the shipped-setup test red.
const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/stub_pistol.weapon.ron"
));
const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/las_carbine.weapon.ron"
));
// GTW-547: Alex Mercer's roster weapon is now the volatile satchel-charge (its on-death Explode
// effect makes the on-death feature LIVE in the shipped skirmish), so the shipped-setup test's
// registry must resolve it too — the shipped `.weapon.ron` read at compile time.
const SHIPPED_VOLATILE_CHARGE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/volatile_charge.weapon.ron"
));

/// Build a registry from the shipped weapon files, keyed by their filename stems —
/// the real-asset registry the shipped `skirmish.ron` setup resolves against (AC5).
/// Returns `None` (assert-fail) if any file fails to parse (no panic in tests).
pub(super) fn shipped_registry() -> Option<WeaponRegistry> {
    let stub_pistol = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    let las_carbine = ron::de::from_str::<WeaponSpec>(SHIPPED_LAS_CARBINE_RON);
    let volatile_charge = ron::de::from_str::<WeaponSpec>(SHIPPED_VOLATILE_CHARGE_RON);
    assert!(
        stub_pistol.is_ok() && las_carbine.is_ok() && volatile_charge.is_ok(),
        "all shipped weapon files must parse: stub_pistol={stub_pistol:?} \
         las_carbine={las_carbine:?} volatile_charge={volatile_charge:?}",
    );
    let (Ok(stub_pistol), Ok(las_carbine), Ok(volatile_charge)) =
        (stub_pistol, las_carbine, volatile_charge)
    else {
        return None;
    };
    Some(WeaponRegistry::new([
        (WeaponName::new("stub_pistol".to_owned()), stub_pistol),
        (WeaponName::new("las_carbine".to_owned()), las_carbine),
        (
            WeaponName::new("volatile_charge".to_owned()),
            volatile_charge,
        ),
    ]))
}

/// The two shipped armor `.armor.ron` files, read at compile time via the same
/// `include_str!` pattern the shipped weapons use — the REAL on-disk authored armor
/// suits (keyed by their filename stems, minus the `.armor` infix), so a GTW-269
/// regression in either file turns the shipped-setup test red.
const SHIPPED_FLAK_VEST_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/armor/flak_vest.armor.ron"
));
const SHIPPED_CARAPACE_PLATE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/armor/carapace_plate.armor.ron"
));

/// Build an armor registry from the shipped armor files, keyed by their filename stems
/// (minus the `.armor` infix) — the real-asset armor registry the shipped
/// `skirmish.ron` setup resolves against (GTW-269, the armor mirror of
/// [`shipped_registry`]). Returns `None` (assert-fail) if either file fails to parse
/// (no panic in tests).
pub(super) fn shipped_armor_registry() -> Option<ArmorRegistry> {
    let flak_vest = ron::de::from_str::<ArmorSpec>(SHIPPED_FLAK_VEST_RON);
    let carapace_plate = ron::de::from_str::<ArmorSpec>(SHIPPED_CARAPACE_PLATE_RON);
    assert!(
        flak_vest.is_ok() && carapace_plate.is_ok(),
        "both shipped armor files must parse: flak_vest={flak_vest:?} carapace_plate={carapace_plate:?}",
    );
    let (Ok(flak_vest), Ok(carapace_plate)) = (flak_vest, carapace_plate) else {
        return None;
    };
    Some(ArmorRegistry::new([
        (ArmorName::new("flak_vest".to_owned()), flak_vest),
        (ArmorName::new("carapace_plate".to_owned()), carapace_plate),
    ]))
}

/// The shipped `toxic_waste_pool.field.ron` catalog entry, read at compile time via the same
/// `include_str!` pattern the shipped armor uses — the REAL on-disk authored field type the
/// shipped `skirmish.ron` seeds (GTW-545), so a regression in the field file (or the skirmish
/// `fields:` reference) turns the shipped-setup test red.
const SHIPPED_TOXIC_POOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/fields/toxic_waste_pool.field.ron"
));

/// Build a field-def catalog from the shipped `assets/content/fields/*.field.ron` files, keyed
/// by their filename stems (minus the `.field` infix) — the real-asset catalog the shipped
/// `skirmish.ron` setup resolves its authored `fields:` placements against (GTW-545, the field
/// mirror of [`shipped_armor_registry`]). Returns `None` (assert-fail) if the file fails to
/// parse (no panic in tests).
pub(super) fn shipped_field_registry() -> Option<crate::fields::FieldDefRegistry> {
    let toxic_pool = ron::de::from_str::<crate::fields::FieldDef>(SHIPPED_TOXIC_POOL_RON);
    assert!(
        toxic_pool.is_ok(),
        "the shipped toxic_waste_pool field file must parse: {toxic_pool:?}",
    );
    let toxic_pool = toxic_pool.ok()?;
    Some(crate::fields::FieldDefRegistry::new([(
        crate::fields::FieldKey::new("toxic_waste_pool".to_owned()),
        toxic_pool,
    )]))
}

/// The three shipped terrain `.terrain.ron` files for the migrated
/// `skirmish.ron` (GTW-396) — the REAL on-disk authored terrain pieces, so a
/// regression in any of these files turns the shipped-setup test red.
/// The UUID-keyed terrain-definition registry the shipped-`skirmish.ron` setup resolves
/// cover/slab UUIDs against (GTW-491).
///
/// The shipped `skirmish.ron` authors NO inline terrain (the GTW-433 procgen migration — its
/// walls / scatter / slabs / floors are all empty), so the setup never RESOLVES a terrain
/// UUID; an EMPTY registry is therefore correct and faithful (it satisfies the
/// `run_setup_with` signature without inventing terrain the shipped file does not author).
/// When procgen drives real terrain (GTW-492+) the registry comes from the shipped UUID-keyed
/// `assets/terrain/<theme>/*.terrain_def.ron` content (GTW-490), resolved by the loader.
#[must_use]
pub(super) fn shipped_terrain_registry() -> TerrainDefRegistry {
    TerrainDefRegistry::default()
}

/// The two shipped gang `.gang.ron` roster files, read at compile time via the same
/// `include_str!` pattern the shipped weapons/armor/terrain use — the REAL on-disk
/// authored gang rosters (keyed by their filename stems, minus the `.gang` infix), so a
/// GTW-414/415 regression in either file turns the shipped-setup test red.
const SHIPPED_GANG_0_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/gangs/gang_0.gang.ron"
));
const SHIPPED_GANG_1_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/gangs/gang_1.gang.ron"
));

/// Build a gang registry from the shipped `assets/content/gangs/*.gang.ron` files, keyed
/// by their filename stems (minus the `.gang` infix) — the real-asset gang registry the
/// shipped `skirmish.ron` setup resolves each placed ganger's `(gang, member)` ref against
/// (GTW-414/415, the gang mirror of [`shipped_terrain_registry`]). Returns `None`
/// (assert-fail) if either file fails to parse (no panic in tests).
pub(super) fn shipped_gang_registry() -> Option<GangRegistry> {
    let gang_0 = ron::de::from_str::<GangRoster>(SHIPPED_GANG_0_RON);
    let gang_1 = ron::de::from_str::<GangRoster>(SHIPPED_GANG_1_RON);
    assert!(
        gang_0.is_ok() && gang_1.is_ok(),
        "both shipped gang files must parse: gang_0={gang_0:?} gang_1={gang_1:?}",
    );
    let (Ok(gang_0), Ok(gang_1)) = (gang_0, gang_1) else {
        return None;
    };
    Some(GangRegistry::new([
        (GangName::new("gang_0".to_owned()), gang_0),
        (GangName::new("gang_1".to_owned()), gang_1),
    ]))
}

/// Run [`setup_battle`] on a fresh `MinimalPlugins` app against the GIVEN registry
/// (the [`run_setup`] variant for the shipped-weapons AC5 path), returning the app +
/// [`BattleSetup`] on success, else assert-failing and returning `None`.
///
/// GTW-491: accepts a `terrain: Option<&TerrainDefRegistry>` and uses the fallback floor
/// cost from `CombatTuning::default().move_costs.open`.
pub(super) fn run_setup_with(
    situation: Situation,
    gangs: GangRegistry,
    registry: WeaponRegistry,
    armor: ArmorRegistry,
    terrain: Option<&TerrainDefRegistry>,
) -> Option<(App, BattleSetup)> {
    // `AssetPlugin` + `ScenePlugin` are required: `setup_battle` now spawns each ganger
    // as a `bsn!` Scene (GTW-322), whose deferred materialization needs the scene/asset
    // infrastructure (the spike's pinned finding).
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    // GTW-384: setup derives each ganger's computed stats from the default stat tuning.
    let stat_tuning = GangerStatTuning::default();
    // GTW-505: the melee weapon registry (holds the `fists` default + the test melee
    // weapon), so each ganger's melee weapon resolves — `None`-authored fixture gangers
    // fall to `fists`.
    let melee = crate::test_support::test_melee_weapon_registry();
    // GTW-396: the fallback floor cost (when terrain is None or default_floor is empty).
    let fallback_floor_cost = crate::tuning::CombatTuning::default().move_costs.open;
    // Clone the registry so the closure can own it (if provided).
    let terrain_clone = terrain.cloned();
    // GTW-545: attach the shipped field-def catalog so a situation authoring a `fields:`
    // placement (the shipped skirmish.ron seeds a toxic pool) resolves it; a situation with no
    // fields is unaffected (an empty FieldRegistry is seeded). Fall back to an empty catalog if
    // the shipped field file failed to parse (fixture-only situations never reference a field).
    let field_defs = shipped_field_registry().unwrap_or_default();
    let outcome = app
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
                    terrain_clone.as_ref(),
                )
                .with_field_defs(&field_defs),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(
        setup.is_some(),
        "setup_battle must succeed on a valid situation + registry",
    );
    let setup = setup?;
    // Drive the `SpawnScene` schedule so the deferred ganger components materialize
    // before the caller queries them (GTW-322): `run_system_once` + `flush()` apply
    // the `spawn_scene` command + reserve the Entity id, but only `app.update()` runs
    // the `SpawnScene` schedule that turns the queued scenes into live components.
    app.update();
    Some((app, setup))
}

/// Build a single-def [`TerrainDefRegistry`] holding the given [`TerrainDef`] under its own
/// key — the terse fixture the GTW-491 net-new presenter-fact tests use to author a wall /
/// slab def with a chosen graphic / footfall (arbitrary discriminators, never a shipped
/// magnitude). The situation's piece UUID is the def's `key`.
pub(super) fn single_def_registry(def: TerrainDef) -> TerrainDefRegistry {
    TerrainDefRegistry::new([(def.key, def)])
}

/// Build a [`GangerAttributes`] record for a [`PlacedGanger`](crate::situation::PlacedGanger)
/// by resolving its `(gang, member)` ref against the canonical
/// [`test_gang_registry`](crate::test_support::test_gang_registry) (GTW-414) — the input
/// the derivation-RELATION assertions feed to
/// [`derive_stats`](crate::ganger::derive_stats) to compute the EXPECTED derived stats
/// for a spawned ganger (GTW-384), so the spawn tests assert the FORMULA, never a pinned
/// magnitude.
///
/// Falls back to a zero-attribute record (assert-fail caught downstream) if the placement
/// does not resolve — every fixture placement IS in the canonical registry, so this
/// never happens on a valid fixture; the explicit fallback keeps the helper
/// `unwrap`/`expect`-free (all denied in tests too).
pub(super) fn attributes_of(placed: &crate::situation::PlacedGanger) -> GangerAttributes {
    let gangs = test_gang_registry();
    let Some(member) = gangs
        .roster(&placed.gang)
        .and_then(|roster| roster.member(&placed.member))
    else {
        // Unreachable for a valid fixture (every placement is in the canonical registry);
        // a zero-attribute fallback keeps the helper panic-free if a test ever drifts.
        return GangerAttributes {
            speed:     crate::ganger::Speed::new(0.0),
            aim:       crate::ganger::Aim::new(0.0),
            strength:  crate::ganger::Strength::new(0.0),
            toughness: Toughness::new(0.0),
            reflexes:  crate::ganger::Reflexes::new(0.0),
            cool:      crate::ganger::Cool::new(0.0),
            grit:      crate::ganger::Grit::new(0.0),
            luck:      Luck::new(0.0),
        };
    };
    member.attributes()
}

/// The C8 minimal fixture: 2 gangers (distinct factions + cells), 1 wall, 1
/// slab — the SAME fixture every C8 assertion reads from. Built over the central
/// [`ganger_at`] / [`wall_at`] builders.
pub(super) fn minimal_fixture() -> (Situation, CellLevel, CellLevel, CellLevel, CellLevel) {
    let alice_at = key(5, 6, 0);
    let bob_at = key(7, 8, 0);
    let wall_cell = key(1, 2, 0);
    let slab_cell = key(3, 4, 1);

    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(alice_at, 0), ganger_at(bob_at, 1)])
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    (situation, alice_at, bob_at, wall_cell, slab_cell)
}

/// Run [`setup_battle`] on a fresh `MinimalPlugins` app, asserting it succeeded,
/// and return the app (so the caller queries the resulting world) plus the
/// [`BattleSetup`] — or assert-fail and return `None` (keeping the tests free of
/// `unwrap`/`expect`/`panic`, all denied in tests too).
///
/// Drives the real `Commands` path via `run_system_once`, then `app.update()`s once
/// to drive the `SpawnScene` schedule so the deferred `bsn!` ganger components
/// materialize before the caller queries them (GTW-322).
///
/// GTW-396: passes the test terrain registry so `wall_at` / `slab_at` authored pieces
/// resolve. Uses the fallback floor cost (`CombatTuning::default().move_costs.open`)
/// since the fixture situations use `SituationBuilder` with no `default_floor`.
pub(super) fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    let terrain = test_terrain_registry();
    run_setup_with(
        situation,
        test_gang_registry(),
        test_registry(),
        test_armor_registry(),
        Some(&terrain),
    )
}

/// The shipped authored situation file, read at compile time via the same
/// `include_str!` pattern `tuning.rs` uses for the shipped `tuning.ron` — the
/// REAL on-disk path (`assets/situations/skirmish.ron`), so a regression in the
/// authored file turns these tests red.
pub(super) const SHIPPED_SITUATION_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/situations/skirmish.ron"
));

/// Parse the shipped `assets/situations/skirmish.ron` into a `Situation`, once,
/// for the AC3/AC4 tests — or assert-fail and return `None` (keeping the tests
/// free of `unwrap`/`expect`/`panic`, all denied in tests too).
pub(super) fn shipped_situation() -> Option<Situation> {
    let parsed = ron::de::from_str::<Situation>(SHIPPED_SITUATION_RON);
    assert!(
        parsed.is_ok(),
        "shipped assets/situations/skirmish.ron must deserialize into Situation: {parsed:?}",
    );
    parsed.ok()
}

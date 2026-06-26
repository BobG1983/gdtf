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
    test_armor_registry, test_terrain_registry, test_weapon_registry as test_registry,
};
pub(super) use crate::{
    armor::{ArmorHardness, ArmorName, ArmorProtection, ArmorRegistry, ArmorSpec, BodyPart, Wears},
    cover::{CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{
        Aiming, Facing, Faction, GangerAttributes, GangerName, Hp, HpMax, LifeState, Luck,
        Position, Shooting, Stance, Toughness, Tu, TuMax, Wounds, WoundsMax, derive_stats,
    },
    inflicted_wound::InflictedWounds,
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    situation::{CoverSpawn, FloorSpawn},
    surface::{SlabState, SurfaceGrid},
    terrain::{
        floor::FloorCostGrid,
        piece::{
            FloorSpec, FootfallSound, StructuralSpec, TerrainGraphicKey, TerrainKindSpec,
            TerrainName, TerrainRegistry, TerrainSpec,
        },
    },
    tuning::{GangerStatTuning, MoveCost},
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireMode, Weapon, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy, Wields},
};

/// The two shipped weapon `.ron` files, read at compile time via the same
/// `include_str!` pattern the shipped situation uses — the REAL on-disk authored
/// weapons (keyed by their filename stems), so an AC5 regression in either file
/// turns the shipped-setup test red.
const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/stub_pistol.weapon.ron"
));
const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/las_carbine.weapon.ron"
));

/// Build a registry from the shipped weapon files, keyed by their filename stems —
/// the real-asset registry the shipped `skirmish.ron` setup resolves against (AC5).
/// Returns `None` (assert-fail) if either file fails to parse (no panic in tests).
pub(super) fn shipped_registry() -> Option<WeaponRegistry> {
    let stub_pistol = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    let las_carbine = ron::de::from_str::<WeaponSpec>(SHIPPED_LAS_CARBINE_RON);
    assert!(
        stub_pistol.is_ok() && las_carbine.is_ok(),
        "both shipped weapon files must parse: stub_pistol={stub_pistol:?} las_carbine={las_carbine:?}",
    );
    let (Ok(stub_pistol), Ok(las_carbine)) = (stub_pistol, las_carbine) else {
        return None;
    };
    Some(WeaponRegistry::new([
        (WeaponName::new("stub_pistol".to_owned()), stub_pistol),
        (WeaponName::new("las_carbine".to_owned()), las_carbine),
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

/// The three shipped terrain `.terrain.ron` files for the migrated
/// `skirmish.ron` (GTW-396) — the REAL on-disk authored terrain pieces, so a
/// regression in any of these files turns the shipped-setup test red.
const SHIPPED_HEAVY_BULKHEAD_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/heavy_bulkhead.terrain.ron"
));
const SHIPPED_BARRICADE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/barricade.terrain.ron"
));
const SHIPPED_DECK_SLAB_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/deck_slab.terrain.ron"
));
const SHIPPED_DECK_FLOOR_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/deck_floor.terrain.ron"
));

/// Build a terrain registry from the shipped terrain piece files — the four
/// pieces `skirmish.ron` now references (`heavy_bulkhead`, `barricade`,
/// `deck_slab`, `deck_floor`), keyed by their filename stems. Returns `None`
/// (assert-fail) if any file fails to parse (no panic in tests).
pub(super) fn shipped_terrain_registry() -> Option<TerrainRegistry> {
    let heavy_bulkhead = ron::de::from_str::<TerrainSpec>(SHIPPED_HEAVY_BULKHEAD_RON);
    let barricade = ron::de::from_str::<TerrainSpec>(SHIPPED_BARRICADE_RON);
    let deck_slab = ron::de::from_str::<TerrainSpec>(SHIPPED_DECK_SLAB_RON);
    let deck_floor = ron::de::from_str::<TerrainSpec>(SHIPPED_DECK_FLOOR_RON);
    assert!(
        heavy_bulkhead.is_ok() && barricade.is_ok() && deck_slab.is_ok() && deck_floor.is_ok(),
        "all shipped terrain files must parse: heavy_bulkhead={heavy_bulkhead:?} \
         barricade={barricade:?} deck_slab={deck_slab:?} deck_floor={deck_floor:?}",
    );
    let (Ok(heavy_bulkhead), Ok(barricade), Ok(deck_slab), Ok(deck_floor)) =
        (heavy_bulkhead, barricade, deck_slab, deck_floor)
    else {
        return None;
    };
    Some(TerrainRegistry::new([
        (
            TerrainName::new("heavy_bulkhead".to_owned()),
            heavy_bulkhead,
        ),
        (TerrainName::new("barricade".to_owned()), barricade),
        (TerrainName::new("deck_slab".to_owned()), deck_slab),
        (TerrainName::new("deck_floor".to_owned()), deck_floor),
    ]))
}

/// Run [`setup_battle`] on a fresh `MinimalPlugins` app against the GIVEN registry
/// (the [`run_setup`] variant for the shipped-weapons AC5 path), returning the app +
/// [`BattleSetup`] on success, else assert-failing and returning `None`.
///
/// GTW-396: now also accepts a `terrain: Option<&TerrainRegistry>` and uses the
/// fallback floor cost from `CombatTuning::default().move_costs.open`.
pub(super) fn run_setup_with(
    situation: Situation,
    registry: WeaponRegistry,
    armor: ArmorRegistry,
    terrain: Option<&TerrainRegistry>,
) -> Option<(App, BattleSetup)> {
    // `AssetPlugin` + `ScenePlugin` are required: `setup_battle` now spawns each ganger
    // as a `bsn!` Scene (GTW-322), whose deferred materialization needs the scene/asset
    // infrastructure (the spike's pinned finding).
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    // GTW-384: setup derives each ganger's computed stats from the default stat tuning.
    let stat_tuning = GangerStatTuning::default();
    // GTW-396: the fallback floor cost (when terrain is None or default_floor is empty).
    let fallback_floor_cost = crate::tuning::CombatTuning::default().move_costs.open;
    // Clone the registry so the closure can own it (if provided).
    let terrain_clone = terrain.cloned();
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                &registry,
                &armor,
                &stat_tuning,
                terrain_clone.as_ref(),
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

/// Build a `Floor`-kind [`TerrainSpec`] carrying the given move cost — the terse
/// fixture the GTW-396 floor-resolution tests use to author a floor piece at a chosen
/// cost (an arbitrary discriminator, never a shipped magnitude).
pub(super) fn floor_spec(cost: u8) -> TerrainSpec {
    TerrainSpec {
        graphic:  TerrainGraphicKey::new("floor".to_owned()),
        footfall: FootfallSound::new("none".to_owned()),
        kind:     TerrainKindSpec::Floor(FloorSpec {
            move_cost: MoveCost::new(cost),
        }),
    }
}

/// Build a `Cover`-kind [`TerrainSpec`] — a LOW-band structural prop with arbitrary
/// stats. Used by the GTW-396 floor-resolution test that points a floor KEY at a
/// NON-`Floor` spec (the `resolve_floor_piece_cost` rejection branch).
pub(super) fn cover_spec() -> TerrainSpec {
    TerrainSpec {
        graphic:  TerrainGraphicKey::new("cover".to_owned()),
        footfall: FootfallSound::new("none".to_owned()),
        kind:     TerrainKindSpec::Cover(StructuralSpec {
            max_hp:           CoverHp::new(30),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        }),
    }
}

/// Build a [`GangerAttributes`] record from an authored [`GangerSpawn`]'s eight
/// attributes — the input the derivation-RELATION assertions feed to
/// [`derive_stats`](crate::ganger::derive_stats) to compute the EXPECTED derived stats
/// for a spawned ganger (GTW-384), so the spawn tests assert the FORMULA, never a pinned
/// magnitude.
pub(super) fn attributes_of(ganger: &GangerSpawn) -> GangerAttributes {
    GangerAttributes {
        speed:     ganger.speed,
        aim:       ganger.aim,
        strength:  ganger.strength,
        toughness: ganger.toughness,
        reflexes:  ganger.reflexes,
        cool:      ganger.cool,
        grit:      ganger.grit,
        luck:      ganger.luck,
    }
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

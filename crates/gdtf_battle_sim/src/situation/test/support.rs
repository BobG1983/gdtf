//! Shared test fixtures + helpers for the `situation` module tests — the
//! `MinimalPlugins` setup drivers, the C8 minimal fixture, and the shipped-asset
//! `include_str!` constants. The reusable ganger / situation / registry builders
//! now live in the crate-central [`crate::test_support`]; this file re-exports them
//! and keeps only the `situation`-module-specific harness. Each concern file does
//! `use super::support::*;` to reach them.

pub(super) use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
};

pub(super) use super::super::*;
// The canonical shared builders + specs + registries + the `(cell, level)` key
// helper (consolidated out of this file's former local copies).
pub(super) use crate::test_support::{
    GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, arbitrary_armor, ganger_at, key,
    test_armor_registry, test_weapon_registry as test_registry, wall_at,
};
pub(super) use crate::{
    armor::{ArmorName, ArmorRegistry, ArmorSpec, BodyPart, WornArmor},
    cover::{CoverLedger, Destroyed},
    ganger::{
        Aiming, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Position, Shooting,
        Stance, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    inflicted_wound::InflictedWounds,
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    surface::{SlabState, SurfaceGrid},
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireMode, Weapon, WeaponName, WeaponRegistry, WeaponSpec},
};

/// The two shipped weapon `.ron` files, read at compile time via the same
/// `include_str!` pattern the shipped situation uses — the REAL on-disk authored
/// weapons (keyed by their filename stems), so an AC5 regression in either file
/// turns the shipped-setup test red.
const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/weapons/stub_pistol.weapon.ron"
));
const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/weapons/las_carbine.weapon.ron"
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
    "/../../assets/armor/flak_vest.armor.ron"
));
const SHIPPED_CARAPACE_PLATE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/armor/carapace_plate.armor.ron"
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

/// Run [`setup_battle`] on a fresh `MinimalPlugins` app against the GIVEN registry
/// (the [`run_setup`] variant for the shipped-weapons AC5 path), returning the app +
/// [`BattleSetup`] on success, else assert-failing and returning `None`.
pub(super) fn run_setup_with(
    situation: Situation,
    registry: WeaponRegistry,
    armor: ArmorRegistry,
) -> Option<(App, BattleSetup)> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &armor, &mut commands)
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(
        setup.is_some(),
        "setup_battle must succeed on a valid situation + registry",
    );
    let setup = setup?;
    app.world_mut().flush();
    Some((app, setup))
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
/// Drives the real `Commands` path via `run_system_once` and flushes the
/// deferred commands via `world.flush()`.
pub(super) fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Run setup as a one-shot system reading the fixture against the central test
    // weapon + armor registries, capturing its result.
    let registry = test_registry();
    let armor = test_armor_registry();
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &armor, &mut commands)
        });

    // The one-shot system itself must run (Ok), and the inner setup must succeed.
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(
        setup.is_some(),
        "setup_battle must succeed on a valid situation",
    );
    let setup = setup?;
    // Flush the deferred Commands (spawns + insert_resource) into the world.
    app.world_mut().flush();
    Some((app, setup))
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

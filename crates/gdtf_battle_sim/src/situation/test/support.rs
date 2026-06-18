//! Shared test fixtures + helpers for the `situation` module tests — the registry
//! builders, the minimal C8 fixture, the `MinimalPlugins` setup drivers, and the
//! shipped-asset `include_str!` constants. Each concern file does
//! `use super::support::*;` to reach them.

pub(super) use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
};

pub(super) use super::super::*;
pub(super) use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, SourceArmor, WornArmor,
    },
    cover::{CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Position,
        Shooting, Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    inflicted_wound::InflictedWounds,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, TerrainKind},
    surface::{SlabState, SurfaceGrid},
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, Weapon,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The weapon KEY every test ganger references — present in [`test_registry`].
pub(super) const TEST_WEAPON_KEY: &str = "test-weapon";

/// Build a `(cell, level)` key from raw coordinates.
pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary [`WeaponSpec`] (NOT shipped magnitudes — mechanism only) carrying
/// a single-shot [`FireMode`] whose one mode is [`ModeKind::Single`], so a
/// resolved bundle proves the [`Weapon`] marker + [`FireMode`] landed.
pub(super) fn arbitrary_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread:   BaseSpread::new(0.25),
        accuracy:      Accuracy::new(1.3),
        kickback:      Kickback::new(0.4),
        fatal_bias:    FatalBias::new(7.0),
        damage:        WeaponDamage::new(12),
        punch:         WeaponPunch::new(5),
        shred:         WeaponShred::new(3),
        damage_type:   DamageType::Kinetic,
        magazine_size: MagazineSize::new(30),
        fire_mode:     FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:        Stable::new(false),
    }
}

/// A registry holding the one [`TEST_WEAPON_KEY`] weapon — the test-built registry
/// the setup resolves each ganger's `weapon` key against (no `AssetServer`).
pub(super) fn test_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        arbitrary_weapon_spec(),
    )])
}

/// The two shipped weapon `.ron` files, read at compile time via the same
/// `include_str!` pattern the shipped situation uses — the REAL on-disk authored
/// weapons (keyed by their filename stems), so an AC5 regression in either file
/// turns the shipped-setup test red.
const SHIPPED_AUTOGUN_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/weapons/autogun.weapon.ron"
));
const SHIPPED_LASGUN_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/weapons/lasgun.weapon.ron"
));

/// Build a registry from the shipped weapon files, keyed by their filename stems —
/// the real-asset registry the shipped `skirmish.ron` setup resolves against (AC5).
/// Returns `None` (assert-fail) if either file fails to parse (no panic in tests).
pub(super) fn shipped_registry() -> Option<WeaponRegistry> {
    let autogun = ron::de::from_str::<WeaponSpec>(SHIPPED_AUTOGUN_RON);
    let lasgun = ron::de::from_str::<WeaponSpec>(SHIPPED_LASGUN_RON);
    assert!(
        autogun.is_ok() && lasgun.is_ok(),
        "both shipped weapon files must parse: autogun={autogun:?} lasgun={lasgun:?}",
    );
    let (Ok(autogun), Ok(lasgun)) = (autogun, lasgun) else {
        return None;
    };
    Some(WeaponRegistry::new([
        (WeaponName::new("autogun".to_owned()), autogun),
        (WeaponName::new("lasgun".to_owned()), lasgun),
    ]))
}

/// Run [`setup_battle`] on a fresh `MinimalPlugins` app against the GIVEN registry
/// (the [`run_setup`] variant for the shipped-weapons AC5 path), returning the app +
/// [`BattleSetup`] on success, else assert-failing and returning `None`.
pub(super) fn run_setup_with(
    situation: Situation,
    registry: WeaponRegistry,
) -> Option<(App, BattleSetup)> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &mut commands)
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

/// An arbitrary roster armor record — distinct per-part magnitudes (NOT shipped
/// tuning) so the seed copy is provably faithful, never asserting a magnitude.
pub(super) fn arbitrary_armor(base: i32) -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored ganger at `at` with the given faction and otherwise
/// arbitrary-but-DISTINCT component values, so a test can prove each field
/// lands on the spawned entity.
pub(super) fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        // A distinct authored name per faction so a spawn test can prove the
        // `GangerName` component lands on the entity (GTW-285).
        name: GangerName::new(format!("Ganger {faction}")),
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Crouching),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        hp_max: HpMax::new(40),
        wounds: Wounds::new(3),
        wounds_max: WoundsMax::new(3),
        tu: Tu::new(60),
        tu_max: TuMax::new(60),
        life_state: LifeState::Alive,
        // The E3.0 attribute stats — distinct arbitrary magnitudes per faction so
        // a per-field readback is provable (NOT shipped tuning; per-ganger data).
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: arbitrary_armor(i32::from(faction) + 1),
        // Every test ganger references the one TEST_WEAPON_KEY in test_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
    }
}

/// An authored wall at `at` with arbitrary cover stats.
pub(super) fn wall_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(
        at,
        TerrainKind::Wall,
        CoverHp::new(120),
        HeightBand::High,
        ArmorProtection::new(8),
        ArmorHardness::new(4),
    )
}

/// The C8 minimal fixture: 2 gangers (distinct factions + cells), 1 wall, 1
/// slab — the SAME fixture every C8 assertion reads from.
pub(super) fn minimal_fixture() -> (Situation, CellLevel, CellLevel, CellLevel, CellLevel) {
    let alice_at = key(5, 6, 0);
    let bob_at = key(7, 8, 0);
    let wall_cell = key(1, 2, 0);
    let slab_cell = key(3, 4, 1);

    let situation = Situation {
        gangers: vec![ganger_at(alice_at, 0), ganger_at(bob_at, 1)],
        walls: vec![wall_at(wall_cell)],
        slabs: vec![slab_cell],
        ..Situation::new()
    };
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

    // Run setup as a one-shot system reading the fixture against the test registry,
    // capturing its result.
    let registry = test_registry();
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &mut commands)
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

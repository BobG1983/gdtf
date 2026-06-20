//! Shared fixtures + helpers the battle-lifecycle test files glob-import — the
//! seeded constants, the authored situations, the `headless_app` harness, the
//! message-buffer drains, and the `world_mut()`-query `LifeState` setters
//! (bevy-traps #7's headless-test carve-out).

pub(super) use bevy::{
    ecs::message::Messages,
    prelude::{App, MinimalPlugins},
};

pub(super) use crate::{
    acts::FireRequested,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType, WornArmor,
    },
    battle::{
        BattleInProgress, BattleLost, BattleReady, BattleRoster, BattleSimPlugin, BattleWon,
        PlayerFaction, SetupBattleRequested, TeardownBattleRequested,
    },
    cover::CoverLedger,
    ganger::{
        Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Shooting,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::CoverDestroyed,
    rng::{BattleSeed, SimRng},
    situation::{BattleSetupError, GangerSpawn, Situation, setup_battle},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// An arbitrary (NOT shipped tuning) seed for a test battle's RNG stream.
pub(super) const SEED: u64 = 0x5A1C_AC75;

/// The weapon KEY every fixture ganger references — present in the registry the
/// `headless_app` inserts (so a setup arms each ganger; GTW-257).
pub(super) const TEST_WEAPON_KEY: &str = "test-weapon";

/// The armor KEY every fixture ganger references — present in the armor registry the
/// `headless_app` inserts (so a setup armors each ganger; GTW-269).
pub(super) const TEST_ARMOR_KEY: &str = "test-armor";

/// An arbitrary [`WeaponSpec`] (NOT shipped magnitudes — mechanism only) for the
/// one [`TEST_WEAPON_KEY`] the fixture gangers reference.
pub(super) fn arbitrary_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.25),
        accuracy:    Accuracy::new(1.0),
        kickback:    Kickback::new(0.4),
        fatal_bias:  FatalBias::new(7.0),
        damage:      WeaponDamage::new(12),
        punch:       WeaponPunch::new(5),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Kinetic,
        magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:      Stable::new(false),
    }
}

/// The test [`WeaponRegistry`] — the one [`TEST_WEAPON_KEY`] weapon the fixture
/// gangers reference, standing in for the app's `Load`-built registry (always
/// present before a battle in the real app).
pub(super) fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        arbitrary_weapon_spec(),
    )])
}

/// The test [`ArmorRegistry`] — the one [`TEST_ARMOR_KEY`] armor suit the fixture
/// gangers reference, standing in for the app's `Load`-built registry (always present
/// before a battle in the real app; GTW-269). `setup_battle_on_request` reads it to
/// armor each ganger.
pub(super) fn armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(TEST_ARMOR_KEY.to_owned()),
        arbitrary_armor(1),
    )])
}

/// Build a `(cell, level)` key from raw coordinates.
pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary armor SPEC (distinct per-part magnitudes, NOT shipped tuning) — the
/// suit the [`TEST_ARMOR_KEY`] resolves to in [`armor_registry`], so a fixture ganger's
/// resolved `WornArmor` is a faithful copy (GTW-269).
pub(super) fn arbitrary_armor(base: i32) -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored ganger at `at` with arbitrary-but-valid component values.
pub(super) fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
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
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        // Every fixture ganger references the one TEST_ARMOR_KEY in armor_registry.
        armor: ArmorName::new(TEST_ARMOR_KEY.to_owned()),
        // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
    }
}

/// A valid two-ganger fixture situation (no cover / slabs / links — link-free
/// validates trivially). Omits `player_faction`, so the struct-level
/// `#[serde(default)]` / [`Default`] supplies [`Faction::default`] = `Faction(0)`
/// (the AC2 default-seed precondition).
pub(super) fn two_ganger_situation() -> Situation {
    Situation {
        gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)],
        ..Situation::new()
    }
}

/// The two-ganger fixture with `player_faction` AUTHORED to gang 1 (overriding the
/// `Faction(0)` default) — the AC3 fixture proving the seed reads
/// `situation.player_faction`, not a hardcoded gang 0.
pub(super) fn two_ganger_situation_player_faction_one() -> Situation {
    Situation {
        player_faction: Faction::new(1),
        ..two_ganger_situation()
    }
}

/// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
/// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and
/// inserts no resource (the `setup_aborts_on_invalid_vertical_link` precedent).
pub(super) fn dangling_link_situation() -> (Situation, VerticalLink) {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored — the link dangles off it
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let situation = Situation {
        gangers: vec![ganger_at(key(0, 0, 0), 0)],
        slabs: vec![present], // only `present` authored; `missing` dangles
        vertical_links: vec![link],
        ..Situation::new()
    };
    (situation, link)
}

/// Build a headless app: [`MinimalPlugins`] (no window / renderer) +
/// [`BattleSimPlugin`] — the `occupancy_sync` / `acts` headless precedent. The sim
/// crate alone, proving NO `gdtf_app` coupling.
///
/// Inserts [`CombatTuning::default`] up front, standing in for E10.4's PERSISTENT
/// `Load` resource (always present in the real app before a battle): once a setup
/// inserts the [`OccupancyGrid`] witness, the gated `Simulate` band runs the bundled
/// dispatch systems, which read `CombatTuning` — so it must be present (the
/// `acts.rs::insert_sim_resources` precedent). It is deliberately NOT one of the
/// battle-lifetime resources the teardown removes.
///
/// Inserts the test [`WeaponRegistry`] + [`ArmorRegistry`] too (GTW-257 / GTW-269):
/// like `CombatTuning` they are PERSISTENT `Load` state present before a battle, and
/// `setup_battle_on_request` reads both to arm + armor each ganger. The fixture gangers
/// reference [`TEST_WEAPON_KEY`] / [`TEST_ARMOR_KEY`], which the registries hold, so a
/// setup succeeds.
pub(super) fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning::default());
    app.insert_resource(weapon_registry());
    app.insert_resource(armor_registry());
    app
}

/// Drain the `BattleReady` buffer and return how many were emitted this run — the
/// setup-complete signal probe. `drain` empties the buffer, so a follow-up call
/// sees only messages written since (the test runs one `update()` then probes).
pub(super) fn drain_battle_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

/// Drain the `BattleWon` buffer and return how many were emitted since the last drain
/// — the win-census probe (mirrors `drain_battle_ready`).
pub(super) fn drain_battle_won(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleWon>>()
        .drain()
        .count()
}

/// Drain the `BattleLost` buffer and return how many were emitted since the last drain
/// — the loss-census probe (mirrors `drain_battle_ready`).
pub(super) fn drain_battle_lost(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleLost>>()
        .drain()
        .count()
}

/// A three-ganger fixture: one player ganger (`faction 0`) + two enemy gangers
/// (`faction 1`) — the AC1/AC2/AC4/AC6(b) win-side fixture (`PlayerFaction(0)` from the
/// `player_faction` default). Link-free, so `setup_battle` validates trivially.
pub(super) fn one_player_two_enemy_situation() -> Situation {
    Situation {
        gangers: vec![
            ganger_at(key(5, 6, 0), 0),
            ganger_at(key(7, 8, 0), 1),
            ganger_at(key(9, 10, 0), 1),
        ],
        ..Situation::new()
    }
}

/// A player-only fixture: every ganger is `faction 0` — the AC6(a) degenerate /
/// empty-enemy-roster fixture (`has_enemy_of` is false, so the census never wins).
pub(super) fn player_only_situation() -> Situation {
    Situation {
        gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 0)],
        ..Situation::new()
    }
}

/// Set the [`LifeState`] of every ganger whose [`Faction`] is `faction` to `to`, via a
/// `world_mut()` query in the test body (bevy-traps #7's headless-test carve-out — NOT
/// a registered system / helper taking `&mut World`). The accepted way to drive a
/// ganger out of the fight without re-running the damage pipeline.
pub(super) fn set_faction_life_state(app: &mut App, faction: u8, to: LifeState) {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target {
            *life = to;
        }
    }
}

/// Set exactly ONE `Alive` ganger of `faction` to `to` (the first the query yields),
/// leaving the rest untouched — the AC2 "only one enemy down, the other still Alive"
/// driver. Returns whether a ganger was found and set (so the caller can assert the
/// fixture is sound). Only flips an `Alive` ganger so repeated calls down DISTINCT
/// gangers (never re-touch one already set).
pub(super) fn set_one_faction_ganger_life_state(app: &mut App, faction: u8, to: LifeState) -> bool {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target && *life == LifeState::Alive {
            *life = to;
            return true;
        }
    }
    false
}

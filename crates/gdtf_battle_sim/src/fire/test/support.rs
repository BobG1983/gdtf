//! Shared test fixtures + the crate-symbol re-exports for the `fire` volley tests.
//!
//! The concern test files (`access` / `fail_closed` / `economy` / `recoil` /
//! `application` / `determinism`) each glob `use super::support::*` to reach these
//! helpers plus the crate types they exercise — the single glob the flat module's
//! combined `use super::*` and `use crate::{…}` block used to provide before the
//! GTW-201 dir-split. The fixtures are RELOCATED VERBATIM (same construction, same
//! magnitudes); no test logic changed.

// Re-exported so each concern file's `use super::support::*` reaches the Bevy harness
// types, the public `fire` surface, and every crate component the tests touch.
pub(super) use bevy::{
    ecs::system::SystemState,
    prelude::{Bundle, Entity, World},
};

pub(super) use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec,
        ArmorType, WornArmor,
    },
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    fire::{BattleGrids, FireOrder, ShooterQuery, TargetQuery, Volley, fire},
    ganger::{
        Aiming, Direction, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, StanceKind,
        Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::{InflictedWound, InflictedWounds},
    magazine::{Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_coarse::ShotKind,
    rng::{BattleSeed, SimRng},
    severity::Severity,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

/// A fixed seed for the per-test RNG streams (an arbitrary value, not tuned).
pub(super) const SEED: u64 = 0xF12E_5EED;

/// Build a `SimRng` from the shared fixed seed (a fresh stream per call).
pub(super) fn rng() -> SimRng {
    SimRng::from_seed(BattleSeed::new(SEED))
}

/// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers.
pub(super) const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A worn suit whose every piece starts at the given stats — arbitrary (NOT
/// shipped) magnitudes so a hit lands in a known regime.
pub(super) fn worn_suit(floor: i32, protection: i32, integrity: i32, hardness: i32) -> WornArmor {
    WornArmor::seed_from(&ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(floor),
        ArmorProtection::new(protection),
        ArmorIntegrity::new(integrity),
        ArmorHardness::new(hardness),
        ArmorType::DEFAULT,
    )))
}

/// The full per-ganger battle-state bundle a target carries (the target query's
/// component set + the worn armor) — arbitrary magnitudes.
pub(super) fn target_bundle(hp: u16, wounds: u8, worn: WornArmor) -> impl Bundle {
    (
        Hp::new(hp),
        Wounds::new(wounds),
        LifeState::Alive,
        worn,
        InflictedWounds::default(),
        Toughness::new(1.0),
        Luck::new(0.0),
    )
}

/// The arbitrary spawn config for a test shooter — its cell, TU pool / max, ammo,
/// fire mode, and aim flag — grouped into one value so `spawn_shooter` stays under
/// the argument-count gate. Magnitudes are arbitrary (not shipped tuning). Not
/// `Copy` — it owns a (now non-`Copy`) [`FireModeSpec`].
#[derive(Clone)]
pub(super) struct ShooterSpec {
    pub(super) x:      i32,
    pub(super) y:      i32,
    pub(super) tu:     u8,
    pub(super) tu_max: u8,
    pub(super) ammo:   u16,
    pub(super) mode:   FireModeSpec,
    pub(super) aiming: bool,
}

/// Spawn an armed shooter facing East at `(spec.x, spec.y, 0)` — carries the full
/// shooter-query component set AND the target-query component set (the shooter is
/// also a ganger, so its own liveness is read from the target query). Arbitrary
/// magnitudes throughout.
pub(super) fn spawn_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        // The bundle carries the test's exact ammo count directly (GTW-275: the
        // WeaponBundle now holds the Magazine grouping, so a separate Magazine in the
        // spawn tuple would be a duplicate-component panic).
        HandlingProfile::new(
            Magazine::new(spec.ammo, mag_size, reload_tu),
            FireMode::new(vec![spec.mode]),
            Stable::new(true),
        ),
    );
    world
        .spawn((
            bundle,
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            // The shooter is also a ganger — it carries the target-query set so
            // its own liveness reads from that query (and it never wounds itself).
            // The target-query set is one nested-tuple bundle so the spawn stays
            // under Bevy's 15-element tuple limit (InflictedWounds is the GTW-279 add).
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id()
}

/// Spawn a shooter whose weapon has a **zero `BaseSpread`** — so every round's
/// dispersion cone is exactly `0` (`θ_cone = base_spread × … = 0`), the cone
/// sampler returns the central axis EXACTLY and consumes no cone draw, and the
/// ONLY thing that perturbs a round's trajectory across the burst is the
/// `prior_shots` recoil-climb tilt (E2.4 `climb_aim_dir`). This isolates the
/// per-round `PriorShots::new(i)` wiring on the real `fire()` path: with the cone
/// pinned to zero, a divergent per-round outcome can come ONLY from the climb.
/// Carries the full shooter + target component sets (the shooter is also a ganger)
/// at `(spec.x, spec.y, 0)` facing East. Arbitrary (not shipped) magnitudes.
pub(super) fn spawn_zero_spread_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.0), // zero cone → trajectory is the climb axis exactly
        Accuracy::new(2.0),
        Kickback::new(0.4), // positive kickback so recoil_growth is engaged
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            // The bundle carries the test's exact ammo count directly (GTW-275: no
            // separate Magazine in the spawn tuple — that would be a duplicate).
            Magazine::new(spec.ammo, mag_size, reload_tu),
            FireMode::new(vec![spec.mode]),
            Stable::new(false), // un-braced so the climb is not damped to nothing
        ),
    );
    world
        .spawn((
            bundle,
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            // The target-query set as one nested-tuple bundle (the GTW-279
            // InflictedWounds add keeps the spawn under the 15-element tuple limit).
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id()
}

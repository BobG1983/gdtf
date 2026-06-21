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
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    fire::{
        BattleGrids, FireOrder, PieceQuery, ShooterQuery, TargetQuery, Volley, WeaponQuery,
        WearsQuery, WieldsQuery, fire,
    },
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
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

/// The six disjoint `fire()` queries bundled into one [`SystemState`] tuple type — the
/// shooter / target / armor-relationship (`Wears`/pieces) / weapon-relationship
/// (`Wields`/weapon) queries `fire()` reads through (GTW-323). A test-local type alias so
/// each concern file spells the wide tuple ONCE (`SystemState::<FireQueries>::new(..)`),
/// keeping the per-test `fire()` setup under clippy's line-count gate.
pub(super) type FireQueries = (
    ShooterQuery<'static, 'static>,
    TargetQuery<'static, 'static>,
    WearsQuery<'static, 'static>,
    PieceQuery<'static, 'static>,
    WieldsQuery<'static, 'static>,
    WeaponQuery<'static, 'static>,
);

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

/// The full per-ganger battle-state bundle a target carries (the target query's
/// component set) — arbitrary magnitudes. Since GTW-323 (ADR-0004) the combat armor
/// lives on related piece entities (spawned via [`equip_uniform_armor`]), NOT on the
/// ganger, so this bundle carries NO armor (GTW-323 slice 3 removed the on-ganger copy).
pub(super) fn target_bundle(hp: u16, wounds: u8) -> impl Bundle {
    (
        Hp::new(hp),
        Wounds::new(wounds),
        LifeState::Alive,
        InflictedWounds::default(),
        Toughness::new(1.0),
        Luck::new(0.0),
    )
}

/// Spawn + relate a ganger's six worn-armor-piece entities (one per [`BodyPart`]),
/// all carrying the SAME uniform stats — the `World`-test equivalent of
/// `setup_battle`'s `queue_spawn_related_scenes::<Wears>` (GTW-323 / ADR-0004).
///
/// `fire()` resolves the struck location through `ganger → Wears → the
/// BodyPart-tagged piece`, so a target spawned for a `fire()` test must carry its
/// piece entities. Spawns each piece with [`WornBy`]`(ganger)` directly — the
/// relationship's insert hook populates the ganger's [`Wears`] collection
/// **synchronously** in a bare `World` (no scene-schedule deferral, so the very next
/// `fire()` call sees the pieces). Uniform stats keep a hit in a known regime; the
/// magnitudes are arbitrary (NOT shipped tuning).
pub(super) fn equip_uniform_armor(
    world: &mut World,
    ganger: Entity,
    floor: i32,
    protection: i32,
    integrity: i32,
    hardness: i32,
) {
    for part in BodyPart::ALL {
        world.spawn((
            WornBy(ganger),
            part,
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            ArmorType::DEFAULT,
        ));
    }
}

/// The loaded round count of a shooter's wielded-weapon magazine — resolved through
/// `ganger → Wields → the weapon entity → Magazine` (GTW-323 slice 2: the [`Magazine`]
/// lives on the weapon entity now, not the ganger). `None` if the ganger wields no
/// weapon or the weapon carries no magazine.
pub(super) fn weapon_rounds(world: &World, ganger: Entity) -> Option<u16> {
    let wields = world.get::<crate::weapon::Wields>(ganger)?;
    let weapon = wields.weapon()?;
    world.get::<Magazine>(weapon).map(|m| *m.rounds())
}

/// Spawn + relate a ganger's wielded-weapon entity from a built [`WeaponBundle`] — the
/// `World`-test equivalent of `setup_battle`'s `queue_spawn_related_scenes::<Wields>`
/// (GTW-323 slice 2 / ADR-0004).
///
/// `fire()` resolves the shooter's weapon through `ganger → Wields → the weapon entity`,
/// so a shooter spawned for a `fire()` test must carry its weapon entity. Spawns the
/// weapon with [`WieldedBy`]`(ganger)` directly — the relationship's insert hook
/// populates the ganger's [`Wields`](crate::weapon::Wields) collection **synchronously**
/// in a bare `World` (no scene-schedule deferral, so the very next `fire()` call sees the
/// weapon), mirroring [`equip_uniform_armor`]. The [`WeaponBundle`] (the GTW-200
/// component set) is inserted on the weapon entity, NOT the ganger.
pub(super) fn equip_weapon(world: &mut World, ganger: Entity, weapon: WeaponBundle) {
    world.spawn((WieldedBy(ganger), weapon));
}

/// Build a test [`WeaponBundle`] from arbitrary (not-shipped) handling/spread numbers —
/// the shared shape `spawn_shooter` / `spawn_zero_spread_shooter` equip their shooters
/// with. `base_spread` and `stable` are the two levers the two spawn variants differ on
/// (a normal vs zero-cone weapon); `ammo` is the test's exact magazine count; `mode` is
/// the offered fire mode (the volley FIRES the mode that rides in the [`FireOrder`], so
/// the offered list is for completeness — the `fire()` path reads the order's mode).
fn test_weapon(
    ammo: u16,
    base_spread: f32,
    kickback: f32,
    stable: bool,
    mode: FireModeSpec,
) -> WeaponBundle {
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(base_spread),
        Accuracy::new(2.0),
        Kickback::new(kickback),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        // The bundle carries the test's exact ammo count directly (GTW-275: the
        // WeaponBundle holds the Magazine grouping, spawned on the weapon entity).
        HandlingProfile::new(
            Magazine::new(ammo, mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(stable),
        ),
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

/// Spawn an armed shooter facing East at `(spec.x, spec.y, 0)` — the ganger carries the
/// full shooter-query component set AND the target-query component set (the shooter is
/// also a ganger, so its own liveness is read from the target query), and its weapon
/// rides on a related **weapon entity** (`Wields`, GTW-323 slice 2). Arbitrary
/// magnitudes throughout.
pub(super) fn spawn_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let ammo = spec.ammo;
    let mode = spec.mode;
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            // The shooter is also a ganger — it carries the target-query set so
            // its own liveness reads from that query (and it never wounds itself).
            // The target-query set is one nested-tuple bundle (InflictedWounds is the
            // GTW-279 add); the shooter's armor lives on related piece entities
            // (GTW-323 slice 1), NOT here.
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    // The shooter's weapon rides on a related weapon entity (GTW-323 slice 2); a normal
    // (non-zero) base spread + a braced weapon, matching the prior on-ganger bundle.
    equip_weapon(world, shooter, test_weapon(ammo, 0.05, 0.2, true, mode));
    // The shooter is a ganger too — equip its worn pieces (GTW-323 slice 1) so a round
    // that strikes it (it never wounds itself, but the query must resolve) finds them.
    equip_uniform_armor(world, shooter, 0, 0, 1, 0);
    shooter
}

/// Spawn a shooter whose weapon has a **zero `BaseSpread`** — so every round's
/// dispersion cone is exactly `0` (`θ_cone = base_spread × … = 0`), the cone
/// sampler returns the central axis EXACTLY and consumes no cone draw, and the
/// ONLY thing that perturbs a round's trajectory across the burst is the
/// `prior_shots` recoil-climb tilt (E2.4 `climb_aim_dir`). This isolates the
/// per-round `PriorShots::new(i)` wiring on the real `fire()` path: with the cone
/// pinned to zero, a divergent per-round outcome can come ONLY from the climb.
/// Carries the full shooter + target component sets (the shooter is also a ganger)
/// at `(spec.x, spec.y, 0)` facing East, its weapon on a related weapon entity
/// (`Wields`). Arbitrary (not shipped) magnitudes.
pub(super) fn spawn_zero_spread_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let ammo = spec.ammo;
    let mode = spec.mode;
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            // The target-query set as one nested-tuple bundle (the GTW-279
            // InflictedWounds add); the shooter's armor lives on related piece
            // entities (GTW-323 slice 1), NOT here.
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    // The zero-cone weapon on a related weapon entity (GTW-323 slice 2): zero base
    // spread (trajectory is the climb axis exactly), positive kickback (recoil_growth
    // engaged), un-braced (so the climb is not damped to nothing).
    equip_weapon(world, shooter, test_weapon(ammo, 0.0, 0.4, false, mode));
    // The shooter is a ganger too — equip its worn pieces (GTW-323 slice 1).
    equip_uniform_armor(world, shooter, 0, 0, 1, 0);
    shooter
}

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
    prelude::{Entity, World},
};

// The canonical fire-mode + target fixtures (GTW-576), consolidated out of this file's
// former local copies.
pub(super) use crate::test_support::{single_mode, target_bundle};
pub(super) use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery,
        Volley, WeaponQuery, WearsQuery, WieldsQuery, fire,
    },
    ganger::{
        Aiming, Direction, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, StanceKind,
        Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::{InflictedWound, InflictedWounds},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{LoadedRounds, Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{AppliedDamage, GangerVerdict, HitReport, HitVerdict},
    resolve_coarse::ShotKind,
    rng::{InjuryRng, SeverityRng, ShotRng},
    severity::Severity,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, FireModeSpec, Handedness, HandlingProfile, Kickback, MagazineSize,
        MeleeDamageProfile, MeleeWeaponBundle, Reach, Shove, Stable, Strikes, TuCost, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy,
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
    // GTW-505 C5: the melee-weapon marker probe `fire()` filters the wielded weapon against.
    MeleeQuery<'static, 'static>,
    // GTW-543: the mounted-weapon marker probe `fire()` PREFERS the wielded weapon against.
    MountedQuery<'static, 'static>,
);

/// A fixed seed for the per-test RNG streams (an arbitrary value, not tuned).
pub(super) const SEED: u64 = 0xF12E_5EED;

/// The boxed [`GangerVerdict`] of a report that LANDED on a live ganger, else `None` —
/// the shared assertion accessor the fire tests read the wound verdict through (a
/// test-side convenience over the closed [`HitVerdict`], not a production probe).
pub(super) fn ganger_verdict(report: &HitReport) -> Option<&GangerVerdict> {
    match &report.verdict {
        HitVerdict::Ganger(verdict) => Some(verdict),
        HitVerdict::Cover(_)
        | HitVerdict::Slab(_)
        | HitVerdict::Ground(_)
        | HitVerdict::NoEffect => None,
    }
}

/// The [`AppliedDamage`] block of a report that LANDED on a live ganger, else `None`.
pub(super) fn applied_of(report: &HitReport) -> Option<AppliedDamage> {
    ganger_verdict(report).map(|verdict| verdict.applied)
}

/// Build a [`ShotRng`] from the shared fixed seed (a fresh stream per call) — composes
/// the canonical [`crate::test_support::shot_rng`] knob (GTW-576).
///
/// The `fire()` path draws from `ShotRng` for cone-sample + body-part-roll; the
/// severity stream is `severity_rng()`. Tests that call `fire()` need both.
pub(super) fn rng() -> ShotRng {
    crate::test_support::shot_rng(SEED)
}

/// Build a [`SeverityRng`] from the shared fixed seed (a fresh stream per call).
pub(super) fn severity_rng() -> SeverityRng {
    crate::test_support::severity_rng(SEED)
}

/// Build an [`InjuryRng`] from the shared fixed seed (a fresh stream per call) — the
/// GTW-438 injury-roll draw stream the `fire()` path threads to `roll_injury`.
pub(super) fn injury_rng() -> InjuryRng {
    crate::test_support::injury_rng(SEED)
}

/// An EMPTY [`InjuryTables`] — the default for `fire()` tests that do not assert an
/// injury (the roll finds no bucket and takes-then-discards its one draw). A test that
/// pins an injury builds a populated table instead.
pub(super) fn injury_tables() -> InjuryTables {
    InjuryTables::default()
}

/// An EMPTY [`InjuryRegistry`] — the default for `fire()` tests that do not assert an
/// injury. A test that pins an injury builds a populated registry instead.
pub(super) fn injury_registry() -> InjuryRegistry {
    InjuryRegistry::default()
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
            WornBy::new(ganger),
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
    world.spawn((WieldedBy::new(ganger), weapon));
}

/// Spawn + relate a MELEE weapon entity on `ganger` (GTW-505 C5) — the `World`-test
/// equivalent of `setup_battle`'s second `queue_spawn_related_scenes::<Wields>` for the
/// melee weapon. Spawns the `MeleeWeaponBundle` (carrying the `MeleeWeapon` marker) with
/// [`WieldedBy`]`(ganger)` directly, so the ganger's [`Wields`](crate::weapon::Wields)
/// collection holds BOTH a ranged and a melee weapon — the exact precondition the
/// zero-ranged-regression test needs (`fire()` must still resolve the RANGED one).
/// Arbitrary (not shipped) magnitudes.
pub(super) fn equip_melee_weapon(world: &mut World, ganger: Entity) {
    let melee = MeleeWeaponBundle::new(
        WeaponName::new("test-melee".to_owned()),
        MeleeDamageProfile::new(
            WeaponDamage::new(9),
            WeaponPunch::new(3),
            WeaponShred::new(8),
            DamageType::Rend,
        ),
        FatalBias::new(4.0),
        Handedness::OneHanded,
        Reach::new(1),
        FightMode::new(vec![FightModeSpec::new(
            FightModeKind::Swing,
            TuCost::new(20),
            Strikes::new(1),
        )]),
        Shove::new(false),
    );
    world.spawn((WieldedBy::new(ganger), melee));
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
            Magazine::new(LoadedRounds::new(ammo), mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(stable),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

/// Spawn + relate a wielded weapon of the given [`Handedness`] on `ganger` (GTW-443) —
/// the hand-count `fire()` tests' equivalent of [`equip_weapon`], differing only in the
/// weapon's handedness (arbitrary ammo / non-zero spread, braced). The
/// `ganger → Wields → weapon` relationship is populated synchronously.
pub(super) fn equip_handed_weapon(world: &mut World, ganger: Entity, handedness: Handedness) {
    let mode = single_mode(0.2, 1);
    let weapon = WeaponBundle::new(
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
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            handedness,
        ),
    );
    world.spawn((WieldedBy::new(ganger), weapon));
}

/// Insert an [`InflictedInjuries`](crate::injuries::InflictedInjuries) ledger on `ganger`
/// carrying ONE [`DisableHand`](crate::injuries::InjuryEffect::DisableHand) injury keyed
/// to `part` (GTW-443) — so its derived [`HandsAvailable`](crate::injuries::HandsAvailable)
/// drops the matching hand. The ledger is the SOLE hand-count source the `fire()` path
/// folds (an absent ledger = the uninjured two-hands default).
pub(super) fn give_disabled_hand(world: &mut World, ganger: Entity, part: BodyPart) {
    use crate::injuries::{GainedInjury, InflictedInjuries, InjuryEffect, InjuryName, InspectText};
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("disabled-hand".to_owned()),
        part,
        Severity::Major,
        vec![InjuryEffect::DisableHand],
        InspectText::new("a disabled hand".to_owned()),
    ));
    world.entity_mut(ganger).insert(ledger);
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

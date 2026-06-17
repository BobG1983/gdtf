//! The E4.5 **`fire()` volley orchestration** — the firing-act integrator, LAST by
//! dependency (`docs/combat/resolution.md` §1 / §1a / §"What's pure math vs sim":
//! "`fire()` owns the economy; every draw from the model RNG").
//!
//! `fire()` runs the whole firing act over **two disjoint Bevy queries** — NO
//! `&mut World` exclusive (the GTW-198 user direction; the GTW-200 weapon
//! decomposition makes the shooter's full state — ganger state + each weapon stat
//! [`Component`](bevy::prelude::Component) — gettable from ONE query, and the struck
//! target gettable from a SECOND query via `get_mut(entity)` where the entity rides
//! out of [`ShotKind::Ganger`]). The act, in order:
//!
//! 1. **Validate** via the shared E4.4 [`can_fire`] guard (the [`FireActor`]
//!    assembled from the queried components, the shooter's [`LifeState`] read from
//!    the TARGET query). If it fails → an **empty** volley, mutating NOTHING (no TU
//!    charge, no draw) — fail-closed (AC2).
//! 2. **Charge** the full mode TU ONCE up front via E4.0 [`spend_tu`]
//!    ([`mode_tu_cost`] = `ModeTuPercent × TuMax × the ×1.5 aim premium when
//!    aiming`), regardless of how many rounds the burst loops (AC3).
//! 3. **Clamp** the burst to ammo ([`clamp_burst`] = `min(ModeShots, Magazine
//!    rounds)`); the [`Magazine`] decrements one round per fired iteration
//!    (saturating, AC4).
//! 4. **Per-round loop** `i in 0..clamped`: compose a [`ShotInputs`] with EVERY
//!    field (shooter pos/facing/stance + target pos/stance + the target cell's
//!    cover band + `cone` = E4.3 [`cone_for`] at `prior_shots = i` + `p` = E2.5
//!    [`concentration_p`] + `prior_shots = PriorShots::new(i)` + `recoil_climb =
//!    tuning.cone_stability.recoil_climb` + `recoil_growth` from
//!    [`stability_for`]), run E2 [`resolve_coarse`], and — only on a
//!    [`ShotKind::Ganger`] — fold E3 [`resolve_and_apply`] onto the struck target
//!    (got from the target query); every non-ganger kind folds to
//!    [`HitReport::no_effect`] (AC5 / AC6). The recoil climbs across the burst and
//!    RESETS between `fire()` calls (each `fire()` starts at `prior_shots = 0`).
//! 5. **Freeze** — returns the `Vec<HitReport>` volley (one report per fired round).
//!
//! ## The two-query disjoint-access design (AC1)
//!
//! The two queries share **no mutable component**, so they coexist without a Bevy
//! `B0001` access conflict: the SHOOTER query holds the read stats + `&mut Tu` +
//! `&mut Magazine` (and **no** `&LifeState`); the TARGET query holds `&mut Hp` /
//! `&mut Wounds` / `&mut LifeState` / `&mut WornArmor` + `&Toughness` + `&Luck`.
//! [`Luck`] is read-only in BOTH (a `&`-vs-`&` overlap is compatible — only a
//! write-vs-read/write of the SAME component conflicts). The shooter's own
//! [`LifeState`] is read from the TARGET query (the shooter is also a ganger →
//! `targets.get(shooter)`), so `&LifeState` never enters the shooter query (which
//! would clash with the target query's `&mut LifeState`).
//!
//! Render-free, deterministic model logic: every random draw bottoms out in the
//! single injected [`SimRng`] (no `thread_rng`, no ad-hoc entropy), so the same
//! [`BattleSeed`](crate::rng::BattleSeed) reproduces a byte-equal volley (AC7); no
//! LOS / fog input is consulted (the presenter boundary). **Zero pixels** — the
//! reports carry only damage / wound math, never a screen coordinate.

use bevy::{
    ecs::{query::With, system::Query},
    prelude::Entity,
};

use crate::{
    aim::{Shooter, cone_for, stability_for},
    cover::CoverLedger,
    ganger::{
        Aiming, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, StanceKind, Toughness, Tu,
        TuMax, Wounds,
    },
    magazine::{FireActor, Magazine, can_fire, clamp_burst, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{HitReport, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotInputs, ShotKind, resolve_coarse},
    rng::SimRng,
    sample_cone::concentration_p,
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Kickback, MagazineSize, Stable,
        Weapon, WeaponDamage, WeaponPunch, WeaponShred, WeaponStats,
    },
};

/// The **shooter query shape** [`fire`] reads the firing entity through — the
/// shooter's ganger state plus each GTW-200 weapon-stat [`Component`](bevy::prelude::Component),
/// `With<Weapon>`.
///
/// A type alias for the wide read+mutate tuple so [`fire`]'s signature stays
/// readable. It is mutable on [`Tu`] (the up-front TU charge) and [`Magazine`] (the
/// per-round decrement) and read-only on everything else; it carries **no**
/// [`LifeState`] — the shooter's liveness is read from the [`TargetQuery`] (whose
/// `&mut LifeState` would clash with a `&LifeState` here). Every other field is the
/// read state [`cone_for`] / [`stability_for`] / [`concentration_p`] /
/// [`mode_tu_cost`] / [`resolve_coarse`] consume, assembled into the borrow-views at
/// the call site.
pub type ShooterQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        // Ganger state the cone / concentration / muzzle composition reads.
        (
            &'static Position,
            &'static Facing,
            &'static Stance,
            &'static Aiming,
            &'static Shooting,
            &'static Luck,
            &'static TuMax,
        ),
        // The mutable firing economy — the up-front TU charge + per-round ammo.
        (&'static mut Tu, &'static mut Magazine),
        // The GTW-200 weapon-stat components the WeaponStats view borrows.
        (
            &'static BaseSpread,
            &'static Accuracy,
            &'static Kickback,
            &'static FatalBias,
            &'static WeaponDamage,
            &'static WeaponPunch,
            &'static WeaponShred,
            &'static DamageType,
            &'static MagazineSize,
            &'static Stable,
        ),
    ),
    With<Weapon>,
>;

/// The **target query shape** [`fire`] folds a hit onto — the struck ganger's four
/// mutable battle surfaces plus the two read attribute stats the severity roll
/// needs.
///
/// A type alias for the disjoint mutable set so [`fire`]'s signature stays readable.
/// It shares **no** mutable component with [`ShooterQuery`] (the shooter writes
/// [`Tu`] / [`Magazine`]; the target writes [`Hp`] / [`Wounds`] / [`LifeState`] /
/// [`WornArmor`](crate::armor::WornArmor)), and [`Luck`] is read-only in both — so
/// the two queries coexist with no `B0001` conflict (AC1). It carries no
/// [`Weapon`] filter (a target need not be armed). The shooter's own liveness is
/// read through this query too (`targets.get(shooter)`), since the shooter is also a
/// ganger.
pub type TargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut crate::armor::WornArmor,
        &'static Toughness,
        &'static Luck,
    ),
>;

/// The **change-driven world grids** [`fire`] marches each round through — bundled
/// into one named ref-struct so [`fire`] stays under clippy's argument-count gate.
///
/// The three grids [`resolve_coarse`] reads (never rebuilds — the change-driven
/// contract; the change-driven sim↔app seam recorded in ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`): the coarse [`OccupancyGrid`], the
/// persistent [`SurfaceGrid`], and the model [`CoverLedger`]. Grouping the cohesive
/// world-state refs into one value (the [`ShotInputs`] / [`TargetGanger`] bundle
/// precedent) keeps [`fire`]'s parameter list under the 8-arg gate. The struct is a
/// transparent borrow record, not itself a wrapped domain scalar; every field is an
/// existing named world-state type (no bare primitive).
#[derive(Debug, Clone, Copy)]
pub struct BattleGrids<'a> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    pub occupancy: &'a OccupancyGrid,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    pub surface:   &'a SurfaceGrid,
    /// The model cover ledger — peeked for the faced cell (stability) and the target
    /// cell's cover band (the aim point).
    pub cover:     &'a CoverLedger,
}

/// The shooter's `Copy` read state, snapshotted **before** the burst loop so the
/// shooter query is only re-borrowed (mutably, for the [`Magazine`] decrement) one
/// round at a time.
///
/// Reading every `Copy` shooter stat into one local up front releases the immutable
/// borrow of the shooter query, so the loop's `shooters.get_mut(shooter)` (the
/// per-round magazine decrement) does not overlap a live read borrow — the
/// bevy-expert's borrow discipline for the two-query design. Every field is an
/// owned named domain value (no bare primitive); it is an internal call-site
/// snapshot, not a wrapped domain scalar.
struct ShooterSnapshot {
    position:    Position,
    facing:      Facing,
    stance:      Stance,
    aiming:      Aiming,
    shooting:    Shooting,
    luck:        Luck,
    base_spread: BaseSpread,
    accuracy:    Accuracy,
    kickback:    Kickback,
    fatal_bias:  FatalBias,
    damage:      WeaponDamage,
    punch:       WeaponPunch,
    shred:       WeaponShred,
    damage_type: DamageType,
    stable:      Stable,
}

impl ShooterSnapshot {
    /// Assemble a transient [`WeaponStats`] borrow-view over this snapshot's weapon
    /// stats — the read-shape the §1/§6 readers ([`cone_for`] / [`resolve_and_apply`])
    /// take, built from the snapshotted components (the query-based equivalent of
    /// [`WeaponBundle::stats`](crate::weapon::WeaponBundle::stats)).
    const fn weapon_stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
        }
    }

    /// Assemble a transient [`Shooter`] borrow-view over this snapshot's ganger state
    /// — the read-shape [`cone_for`] / [`stability_for`] take.
    const fn shooter_view(&self) -> Shooter<'_> {
        Shooter {
            stance:   &self.stance,
            aiming:   &self.aiming,
            position: &self.position,
            facing:   &self.facing,
        }
    }
}

/// The **firing order** — what the shooter is firing and where (`docs/combat/resolution.md`
/// §1: the per-action selected fire mode + the aim cell).
///
/// The act args bundled into one named value (the [`ShotInputs`] / [`BattleGrids`]
/// bundle precedent) so [`fire`] stays under clippy's argument-count gate: the
/// selected [`FireModeSpec`], the aim [`Cell`], and the aim [`Level`]. A transparent
/// argument record, not itself a wrapped domain scalar; every field is an existing
/// named domain type.
#[derive(Debug, Clone, Copy)]
pub struct FireOrder<'a> {
    /// The selected fire mode's per-mode numbers (cone mult / TU% / shot count).
    pub mode:         &'a FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}

/// The **target geometry** every round in the burst aims at — composed once (it is
/// constant across the burst) and threaded into each round's [`ShotInputs`].
///
/// The target position is the player's aim `(cell, level)`; its stance defaults to
/// [`StanceKind::Standing`] — the locked TARGET query (the disjoint mutable set, AC1)
/// carries **no** `Stance` to read, so the documented default stands in; the cover
/// band is the model ledger's entry at the target cell (so a deliberately-shot crate
/// aims at its own band midpoint), or `None` for a bare ganger target. Every field is
/// an existing named domain value (no bare primitive).
#[derive(Debug, Clone, Copy)]
struct TargetGeometry {
    position:   Position,
    stance:     Stance,
    cover_band: Option<crate::cover::HeightBand>,
}

impl TargetGeometry {
    /// Compose the target geometry once from the aim `(cell, level)` and the model
    /// cover ledger (peeked for the target cell's band — never rebuilt).
    fn compose(target_cell: Cell, target_level: Level, cover: &CoverLedger) -> Self {
        let at = CellLevel::new(target_cell, target_level);
        Self {
            position:   Position::new(at),
            stance:     Stance::new(StanceKind::Standing),
            cover_band: cover.peek(&at).map(|entry| entry.height_band),
        }
    }
}

/// Read the shooter's `Copy` state off the shooter query into a [`ShooterSnapshot`]
/// plus the [`FireActor`]-shaping economy reads — releasing the query's immutable
/// borrow before [`fire`]'s mutable re-borrows.
///
/// Returns `None` when the shooter is not in the query (unarmed / despawned), so
/// [`fire`] fails closed. The `(Tu, TuMax, Aiming, Magazine, Luck)` tuple is the
/// economy state [`can_fire`] / [`mode_tu_cost`] / [`resolve_and_apply`] read.
fn read_shooter(
    shooter: Entity,
    shooters: &ShooterQuery,
) -> Option<(ShooterSnapshot, Tu, TuMax, Aiming, Magazine)> {
    let reads = shooters.get(shooter).ok()?;
    let (
        (position, facing, stance, aiming, shooting, luck, tu_max),
        (tu, magazine),
        (
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage,
            punch,
            shred,
            damage_type,
            _magazine_size,
            stable,
        ),
    ) = reads;
    let snapshot = ShooterSnapshot {
        position:    *position,
        facing:      *facing,
        stance:      *stance,
        aiming:      *aiming,
        shooting:    *shooting,
        luck:        *luck,
        base_spread: *base_spread,
        accuracy:    *accuracy,
        kickback:    *kickback,
        fatal_bias:  *fatal_bias,
        damage:      *damage,
        punch:       *punch,
        shred:       *shred,
        damage_type: *damage_type,
        stable:      *stable,
    };
    Some((snapshot, *tu, *tu_max, *aiming, *magazine))
}

/// The **constant-per-burst inputs** to [`resolve_round`] — the shooter snapshot, the
/// target geometry, and the selected fire mode, bundled so [`resolve_round`] stays
/// under clippy's argument-count gate.
///
/// These three are the same for every round in the burst (only `prior_shots` and the
/// RNG cursor advance per round), so grouping them as one borrow record (the
/// [`ShotInputs`] / [`BattleGrids`] bundle precedent) keeps the per-round verb's
/// parameter list small. A transparent argument record, not itself a wrapped domain
/// scalar.
#[derive(Clone, Copy)]
struct RoundSetup<'a> {
    snapshot: &'a ShooterSnapshot,
    geometry: TargetGeometry,
    mode:     &'a FireModeSpec,
}

/// Resolve **one round** of the burst — compose its [`ShotInputs`], run E2
/// [`resolve_coarse`], and fold E3 [`resolve_and_apply`] onto the struck ganger.
///
/// Composes every [`ShotInputs`] field (AC5 / AC8): the shooter's pos/facing/stance,
/// the target geometry, `cone` = [`cone_for`] at `prior_shots`, `p` =
/// [`concentration_p`], `recoil_climb` = the `tuning.cone_stability.recoil_climb`
/// leaf, and `recoil_growth` from [`stability_for`]. A [`ShotKind::Ganger`] outcome
/// folds via [`resolve_and_apply`] onto the struck target (got from the TARGET query
/// — a struck entity that is not a queryable target, or any non-ganger kind, folds to
/// [`HitReport::no_effect`], never a panic). Every draw is from the injected
/// [`SimRng`].
fn resolve_round(
    setup: RoundSetup,
    prior_shots: crate::cone::PriorShots,
    grids: BattleGrids,
    targets: &mut TargetQuery,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> HitReport {
    let snapshot = setup.snapshot;
    let geometry = setup.geometry;
    let shooter_view = snapshot.shooter_view();
    let cone = cone_for(
        &shooter_view,
        snapshot.weapon_stats(),
        setup.mode,
        prior_shots,
        grids.cover,
        tuning,
    );
    let (_cone_mult, recoil_growth) =
        stability_for(&shooter_view, snapshot.stable, grids.cover, tuning);
    let p = concentration_p(
        snapshot.shooting,
        snapshot.accuracy,
        tuning.cone_stability.concentration,
    );

    let shot = ShotInputs {
        shooter_position: snapshot.position,
        shooter_facing: snapshot.facing,
        shooter_stance: snapshot.stance,
        target_position: geometry.position,
        target_stance: geometry.stance,
        cover_band: geometry.cover_band,
        cone,
        p,
        prior_shots,
        recoil_climb: tuning.cone_stability.recoil_climb,
        recoil_growth,
    };

    let outcome = resolve_coarse(
        &shot,
        grids.occupancy,
        grids.surface,
        grids.cover,
        tuning,
        rng,
    );

    match outcome.kind {
        ShotKind::Ganger(struck) => match targets.get_mut(struck) {
            Ok((mut hp, mut wounds, mut life, mut worn, toughness, target_luck)) => {
                resolve_and_apply(
                    &outcome,
                    snapshot.weapon_stats(),
                    snapshot.luck,
                    TargetGanger {
                        hp:        &mut hp,
                        wounds:    &mut wounds,
                        life:      &mut life,
                        worn:      &mut worn,
                        toughness: *toughness,
                        luck:      *target_luck,
                    },
                    struck,
                    tuning,
                    rng,
                )
            }
            Err(_) => HitReport::no_effect(outcome.kind),
        },
        other => HitReport::no_effect(other),
    }
}

/// Run the whole firing act and return its **frozen volley** — the E4.5 capstone
/// integrator (`docs/combat/resolution.md` §1 / §1a; the authoritative-model role
/// this crate plays in the model/view split, ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A proper query-based Bevy function — **NO `&mut World`** (AC1): the SHOOTER
/// query ([`ShooterQuery`], `With<Weapon>`) carries the shooter's ganger state plus
/// each GTW-200 weapon-stat component (mutable on [`Tu`] + [`Magazine`]); the TARGET
/// query ([`TargetQuery`]) carries the struck ganger's four mutable battle surfaces
/// plus the two read attribute stats — a disjoint mutable set so the two coexist
/// (only [`Luck`], read in both, overlaps, and a `&`-vs-`&` overlap is compatible).
/// The act:
///
/// 1. **Validate** ([`can_fire`], the shooter's [`LifeState`] read from the target
///    query): on failure → an EMPTY volley, mutating NOTHING (AC2, fail-closed).
/// 2. **Charge** the full mode TU ONCE via [`spend_tu`] ([`mode_tu_cost`]) — once
///    regardless of round count (AC3).
/// 3. **Clamp** the burst to ammo ([`clamp_burst`]); the [`Magazine`] decrements one
///    round per fired iteration (saturating, AC4).
/// 4. **Per-round loop** ([`resolve_round`]): compose every [`ShotInputs`] field, run
///    [`resolve_coarse`], and fold [`resolve_and_apply`] onto the struck target (got
///    via `targets.get_mut(entity)`) on a [`ShotKind::Ganger`] — else
///    [`HitReport::no_effect`] (AC5 / AC6). Recoil climbs across the burst
///    (`prior_shots = i`) and RESETS between `fire()` calls.
///
/// Every draw bottoms out in the injected [`SimRng`] (no `thread_rng` / ad-hoc
/// entropy), so the same [`BattleSeed`](crate::rng::BattleSeed) reproduces a
/// byte-equal volley (AC7); no LOS / fog is consulted (the presenter boundary). The
/// aim cell + selected mode ride in `order` ([`FireOrder`]); the world grids in
/// `grids` ([`BattleGrids`]). **Zero pixels** — the reports carry only damage / wound
/// math.
///
/// Returns an empty `Vec` (and mutates nothing) when [`can_fire`] fails, the shooter
/// entity is not in the shooter query, or the magazine is empty. The struck entity
/// not being a queryable target (e.g. it lacks a target component) folds that round
/// to [`HitReport::no_effect`] — never a panic.
#[must_use]
pub fn fire(
    shooter: Entity,
    order: FireOrder,
    shooters: &mut ShooterQuery,
    targets: &mut TargetQuery,
    grids: BattleGrids,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> Vec<HitReport> {
    // (1) Snapshot the shooter's Copy read state up front (the immutable borrow is
    //     released before the mutable re-borrows). A shooter not in the shooter query
    //     (unarmed / despawned) fires nothing.
    let Some((snapshot, shooter_tu, shooter_tu_max, shooter_aiming, magazine_now)) =
        read_shooter(shooter, shooters)
    else {
        return Vec::new();
    };

    // The shooter's own liveness is read from the TARGET query (the shooter is also
    // a ganger), so &LifeState never enters the shooter query (it would clash with
    // the target query's &mut LifeState). A shooter with no target-query components
    // cannot be validated as alive → fail-closed (empty volley, no mutation).
    let Ok((_, _, shooter_life, ..)) = targets.get(shooter) else {
        return Vec::new();
    };
    let shooter_life = *shooter_life;

    // (1) VALIDATE via the shared can_fire guard (AC2). On failure, return an empty
    //     volley having mutated NOTHING — no TU charge, no draw (fail-closed).
    let actor = FireActor {
        life:     &shooter_life,
        tu:       &shooter_tu,
        tu_max:   &shooter_tu_max,
        aiming:   &shooter_aiming,
        magazine: &magazine_now,
    };
    if !can_fire(
        &actor,
        order.mode,
        order.target_cell,
        order.target_level,
        tuning,
    ) {
        return Vec::new();
    }

    // (2) CHARGE the full mode TU ONCE up front (AC3) — debit the shooter's Tu via
    //     the shared mode_tu_cost (the same source can_fire's affordability read).
    let charge = mode_tu_cost(order.mode, &shooter_tu_max, &shooter_aiming, tuning);
    if let Ok((_, (mut tu_mut, _), _)) = shooters.get_mut(shooter) {
        spend_tu(&mut tu_mut, charge);
    } else {
        // Unreachable after the get() above succeeded, but stay panic-free.
        return Vec::new();
    }

    // (3) CLAMP the burst to the rounds actually loaded (AC4): min(ModeShots, ammo).
    let rounds = *clamp_burst(order.mode.shots, &magazine_now);

    // (4) The target geometry, composed once (constant across the burst); the
    //     constant-per-burst inputs bundled for the per-round verb.
    let geometry = TargetGeometry::compose(order.target_cell, order.target_level, grids.cover);
    let setup = RoundSetup {
        snapshot: &snapshot,
        geometry,
        mode: order.mode,
    };

    // (5) PER-ROUND LOOP. Recoil climbs with prior_shots = i (the round index), and
    //     resets between fire() calls because i restarts at 0 every call.
    let mut reports = Vec::with_capacity(usize::from(rounds));
    for i in 0..rounds {
        let report = resolve_round(
            setup,
            crate::cone::PriorShots::new(i),
            grids,
            targets,
            tuning,
            rng,
        );

        // Decrement the magazine one round per fired iteration (saturating, AC4) —
        // re-borrow the shooter query mutably (disjoint from the target get_mut above).
        if let Ok((_, (_, mut mag_mut), _)) = shooters.get_mut(shooter) {
            mag_mut.spend_round();
        }

        reports.push(report);
    }

    reports
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::SystemState, prelude::World};

    use super::*;
    use crate::{
        armor::{
            ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
            SourceArmor, WornArmor,
        },
        cover::{CoverEntry, CoverHp, HeightBand},
        ganger::Direction,
        metric::{Cell, CellLevel, Level},
        occupancy::OccupancyGrid,
        rng::BattleSeed,
        surface::SurfaceGrid,
        weapon::{
            DamageProfile, FireMode, HandlingProfile, ModeConeMult, ModeKind, ModeShots,
            ModeTuPercent, WeaponBundle, WeaponName,
        },
    };

    /// A fixed seed for the per-test RNG streams (an arbitrary value, not tuned).
    const SEED: u64 = 0xF12E_5EED;

    /// Build a `SimRng` from the shared fixed seed (a fresh stream per call).
    fn rng() -> SimRng {
        SimRng::from_seed(BattleSeed::new(SEED))
    }

    /// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers.
    const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(tu_percent),
            ModeShots::new(shots),
        )
    }

    /// A worn suit whose every piece starts at the given stats — arbitrary (NOT
    /// shipped) magnitudes so a hit lands in a known regime.
    fn worn_suit(floor: i32, protection: i32, integrity: i32, hardness: i32) -> WornArmor {
        WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            ArmorType::DEFAULT,
        )))
    }

    /// The full per-ganger battle-state bundle a target carries (the target query's
    /// component set + the worn armor) — arbitrary magnitudes.
    fn target_bundle(hp: u16, wounds: u8, worn: WornArmor) -> impl bevy::prelude::Bundle {
        (
            Hp::new(hp),
            Wounds::new(wounds),
            LifeState::Alive,
            worn,
            Toughness::new(1.0),
            Luck::new(0.0),
        )
    }

    /// The arbitrary spawn config for a test shooter — its cell, TU pool / max, ammo,
    /// fire mode, and aim flag — grouped into one value so `spawn_shooter` stays under
    /// the argument-count gate. Magnitudes are arbitrary (not shipped tuning). Not
    /// `Copy` — it owns a (now non-`Copy`) [`FireModeSpec`].
    #[derive(Clone)]
    struct ShooterSpec {
        x:      i32,
        y:      i32,
        tu:     u8,
        tu_max: u8,
        ammo:   u16,
        mode:   FireModeSpec,
        aiming: bool,
    }

    /// Spawn an armed shooter facing East at `(spec.x, spec.y, 0)` — carries the full
    /// shooter-query component set AND the target-query component set (the shooter is
    /// also a ganger, so its own liveness is read from the target query). Arbitrary
    /// magnitudes throughout.
    fn spawn_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
        let mag_size = MagazineSize::new(30);
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
            HandlingProfile::new(mag_size, FireMode::new(vec![spec.mode]), Stable::new(true)),
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
                Magazine::new(spec.ammo, mag_size),
                // The shooter is also a ganger — it carries the target-query set so
                // its own liveness reads from that query (and it never wounds itself).
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                Toughness::new(1.0),
                Luck::new(0.0),
            ))
            .id()
    }

    /// AC1 — the two-query design is access-compatible: a `SystemState` over
    /// `(ShooterQuery, TargetQuery)` constructs WITHOUT a `B0001` conflict panic.
    /// Building the `SystemState` validates the access set, so this IS the regression
    /// check that the shooter (mut Tu/Magazine, no `LifeState`) and target (mut
    /// Hp/Wounds/`LifeState`/`WornArmor`) queries share no conflicting mutable
    /// component.
    #[test]
    fn two_queries_are_access_compatible_no_b0001() {
        let mut world = World::new();
        // If the two queries had a conflicting mutable access, SystemState::new would
        // panic here (the param-validation B0001 check). Reaching the assert proves
        // the design is disjoint.
        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let (_shooters, _targets) = state.get_mut(&mut world);
        // Reaching here means the access set validated — the two queries coexist.
    }

    /// AC2 — fail-closed when the magazine is empty: an empty volley, NO TU charge,
    /// NO draw, NO mutation.
    #[test]
    fn empty_magazine_fires_nothing_and_mutates_nothing() {
        let mut world = World::new();
        let tuning = CombatTuning::default();
        let mode = single_mode(0.2, 3);
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 5,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 0,
                mode,
                aiming: false,
            },
        );

        let occupancy = OccupancyGrid::new();
        let surface = SurfaceGrid::new();
        let cover = CoverLedger::new();
        let mut r = rng();
        let mut fresh = rng();

        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let reports = {
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(8, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };

        assert!(reports.is_empty(), "an empty magazine must fire nothing");
        // No draw was taken — the used RNG matches a fresh stream's next draw.
        assert_eq!(
            r.next_u64(),
            fresh.next_u64(),
            "no draw on a fail-closed fire"
        );
        // No TU charged — the pool is unchanged.
        let tu = world.get::<Tu>(shooter).copied();
        assert_eq!(
            tu,
            Some(Tu::new(200)),
            "no TU charged on a fail-closed fire"
        );
    }

    /// AC2 — fail-closed when the shooter is not Alive (Downed): an empty volley,
    /// no charge, no draw.
    #[test]
    fn dead_shooter_fires_nothing() {
        let mut world = World::new();
        let tuning = CombatTuning::default();
        let mode = single_mode(0.2, 3);
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 5,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 10,
                mode,
                aiming: false,
            },
        );
        // Down the shooter — can_fire requires Alive.
        if let Some(mut life) = world.get_mut::<LifeState>(shooter) {
            *life = LifeState::Downed;
        }

        let occupancy = OccupancyGrid::new();
        let surface = SurfaceGrid::new();
        let cover = CoverLedger::new();
        let mut r = rng();

        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let reports = {
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(8, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };

        assert!(reports.is_empty(), "a downed shooter must fire nothing");
        let tu = world.get::<Tu>(shooter).copied();
        assert_eq!(tu, Some(Tu::new(200)), "no TU charged for a downed shooter");
        let mag = world.get::<Magazine>(shooter).map(|m| **m);
        assert_eq!(mag, Some(10), "ammo unchanged for a downed shooter");
    }

    /// AC3 — the TU charge is taken ONCE up front regardless of round count, and
    /// reflects the ×1.5 aim premium (aiming costs strictly more than hip-fire).
    #[test]
    fn charge_is_taken_once_and_reflects_aiming() {
        let tuning = CombatTuning::default();
        let mode = single_mode(0.3, 5); // a 5-round burst

        // The expected charge (the shared mode_tu_cost), hip vs aimed.
        let hip_charge = mode_tu_cost(&mode, &TuMax::new(100), &Aiming::new(false), &tuning);
        let aim_charge = mode_tu_cost(&mode, &TuMax::new(100), &Aiming::new(true), &tuning);
        assert!(
            *aim_charge > *hip_charge,
            "aiming must cost strictly more TU"
        );

        // Fire a 5-round burst hip-fired and assert Tu dropped by EXACTLY one charge.
        for (aiming, expected_charge) in [(false, hip_charge), (true, aim_charge)] {
            let mut world = World::new();
            let shooter = spawn_shooter(
                &mut world,
                ShooterSpec {
                    x: 5,
                    y: 5,
                    tu: 200,
                    tu_max: 100,
                    ammo: 30,
                    mode,
                    aiming,
                },
            );
            let occupancy = OccupancyGrid::new();
            let surface = SurfaceGrid::new();
            let cover = CoverLedger::new();
            let mut r = rng();

            let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
            {
                let (mut shooters, mut targets) = state.get_mut(&mut world);
                // off into empty space → all misses, fires the full clamped burst
                let reports = fire(
                    shooter,
                    FireOrder {
                        mode:         &mode,
                        target_cell:  Cell::new(40, 5),
                        target_level: Level::new(0),
                    },
                    &mut shooters,
                    &mut targets,
                    BattleGrids {
                        occupancy: &occupancy,
                        surface:   &surface,
                        cover:     &cover,
                    },
                    &tuning,
                    &mut r,
                );
                assert_eq!(reports.len(), 5, "the full 5-round burst fired");
            }

            let tu = world.get::<Tu>(shooter).map(|t| **t);
            assert_eq!(
                tu,
                Some(200u8.saturating_sub(*expected_charge)),
                "Tu must drop by exactly ONE mode charge across the whole burst (aiming={aiming})",
            );
        }
    }

    /// AC4 — ammo clamp: a burst of more shots than the magazine holds fires exactly
    /// Magazine-many rounds and ends with the magazine at 0.
    #[test]
    fn ammo_clamps_the_burst_and_drains_the_magazine() {
        let mut world = World::new();
        let tuning = CombatTuning::default();
        let mode = single_mode(0.1, 8); // wants 8 shots
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 5,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 3, // only 3 loaded
                mode,
                aiming: false,
            },
        );

        let occupancy = OccupancyGrid::new();
        let surface = SurfaceGrid::new();
        let cover = CoverLedger::new();
        let mut r = rng();

        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let reports = {
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(40, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };

        assert_eq!(
            reports.len(),
            3,
            "the burst is clamped to the 3 loaded rounds"
        );
        let mag = world.get::<Magazine>(shooter).map(|m| **m);
        assert_eq!(mag, Some(0), "the magazine ends drained to 0");
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
    fn spawn_zero_spread_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
        let mag_size = MagazineSize::new(30);
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
                mag_size,
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
                Magazine::new(spec.ammo, mag_size),
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                Toughness::new(1.0),
                Luck::new(0.0),
            ))
            .id()
    }

    /// AC5 — recoil climbs across the burst (per-round `prior_shots = i`) and RESETS
    /// between `fire()` calls — proven ON THE REAL `fire()` PATH (it CALLS `fire()`),
    /// not by re-composing the cone with a helper.
    ///
    /// `prior_shots` is load-bearing on `fire()`'s real output twice over: it widens
    /// `cone_for` AND tilts the `climb_aim_dir` central axis upward. Here the weapon's
    /// `BaseSpread` is `0`, so the cone is exactly `0` every round and the ONLY thing
    /// that moves a round is the climb tilt. A LOW-band target sits in line; the aim
    /// point is pinned LOW (the target cell carries a LOW cover band), so round 0
    /// (zero tilt) flies level and IMPACTS the LOW occupant. With a deliberately large
    /// `recoil_climb`, the later rounds' axis tilts up enough to sail OVER the LOW
    /// occupant — so the volley is NOT all-`Ganger`. A regression that hardcoded
    /// `PriorShots::new(0)` every round would fly every round level and strike the
    /// target on EVERY round (a uniform all-`Ganger` volley), so it FAILS this test.
    ///
    /// RESET: a SECOND `fire()` call over an identical fresh world + identical fresh
    /// seed must reproduce the SAME round-0 outcome (and the same whole volley),
    /// proving each call starts at `prior_shots = 0` — observed across two real
    /// `fire()` invocations, not by re-evaluating one pure closure at arg 0 twice.
    #[test]
    fn recoil_climbs_across_burst_and_resets_between_calls() {
        // A large recoil_climb so the per-round upward tilt is unmistakable — tests
        // are free to author arbitrary tuning (the central_axis.rs precedent).
        let mut tuning = CombatTuning::default();
        tuning.cone_stability.recoil_climb = crate::tuning::RecoilClimb::new(2.0);
        let mode = single_mode(0.1, 3); // a 3-round burst

        // Build an identical world + a LOW-band in-line target each call, fire a
        // 3-round burst at it, and return the frozen volley.
        let run = || {
            let mut world = World::new();
            let shooter = spawn_zero_spread_shooter(
                &mut world,
                ShooterSpec {
                    x: 2,
                    y: 5,
                    tu: 200,
                    tu_max: 100,
                    ammo: 10,
                    mode,
                    aiming: true,
                },
            );
            // A LOW-band ganger occupant directly East at (8,5,0) — deep HP/Wounds and
            // real armor so round 0 wounds but does NOT kill it. That way the volley's
            // later rounds turning non-`Ganger` is unambiguously the recoil climb
            // (they sail OVER), not corpse-skip turning a struck round into no-effect.
            let target = world
                .spawn(target_bundle(500, 60, worn_suit(20, 60, 200, 10)))
                .id();
            let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
            let mut occupancy = OccupancyGrid::new();
            occupancy.set_occupant(target_at, Some(target));
            occupancy.set_occupant_band(target_at, Some(HeightBand::Low));
            let surface = SurfaceGrid::new();
            // A LOW cover band at the target cell pins fire()'s aim point LOW (the aim
            // point reads the target cell's cover band), so round 0 flies level into
            // the LOW occupant. The cover is LOW too, so a round that sails over the
            // LOW occupant also sails over it — no ambiguity.
            let mut cover = CoverLedger::new();
            cover.insert(
                target_at,
                CoverEntry::seeded(
                    CoverHp::new(10),
                    HeightBand::Low,
                    ArmorProtection::new(0),
                    ArmorHardness::new(0),
                ),
            );
            let mut r = rng();
            let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            let reports = fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(8, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            );
            (reports, target)
        };

        let (volley, target) = run();
        assert_eq!(volley.len(), 3, "the full 3-round burst fired");

        // Round 0 (zero prior shots → level flight) IMPACTS the LOW occupant.
        let Some(first) = volley.first() else {
            return;
        };
        assert_eq!(
            first.kind,
            ShotKind::Ganger(target),
            "round 0 (zero tilt) must strike the in-line LOW target",
        );
        assert!(
            first.applied.is_some(),
            "round 0's ganger hit must carry an AppliedDamage block",
        );

        // The CLIMB is load-bearing: with prior_shots wired per round, the later
        // rounds tilt UP and sail OVER the LOW occupant, so the volley is NOT all
        // `Ganger(target)`. A fixed-PriorShots::new(0) regression would strike the
        // target on every round (a uniform volley) and FAIL this assertion.
        let all_strike_target = volley
            .iter()
            .all(|report| report.kind == ShotKind::Ganger(target));
        assert!(
            !all_strike_target,
            "the recoil climb must lift later rounds off the LOW target — a fixed \
             prior_shots=0 would strike every round: {volley:?}",
        );

        // RESET — a second fire() call over an identical fresh world + fresh seed
        // reproduces the SAME round-0 outcome (each call starts at prior_shots = 0).
        let (volley_again, _) = run();
        let Some(first_again) = volley_again.first() else {
            return;
        };
        assert_eq!(
            first, first_again,
            "a second fire() call must reproduce round 0 — recoil resets to prior_shots=0",
        );
        // The whole volley reproduces too (the per-round climb restarts at 0).
        assert_eq!(
            volley, volley_again,
            "a second fire() call must reproduce the whole volley (recoil resets)",
        );
    }

    /// AC6 — application: firing at an in-line target yields a `HitReport` with
    /// `ShotKind::Ganger(entity)` + an `AppliedDamage` block, and the target's
    /// Hp/Wounds changed in the world; while a fire into empty space yields only
    /// `no_effect` reports and the (absent) target is untouched.
    #[test]
    fn fire_at_in_line_target_applies_damage() {
        let mut world = World::new();
        let tuning = CombatTuning::default();
        let mode = single_mode(0.2, 1);
        // Shooter at (2,5) facing East; aiming for a tight cone onto the target.
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 2,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 10,
                mode,
                aiming: true,
            },
        );

        // A standing (HIGH-band) target directly East at (8,5,0). fire()'s aim point
        // uses the documented default Standing target stance (the locked target query
        // carries no Stance), so the round flies at the standing silhouette; a HIGH
        // occupant band is equal-or-lower than any round band → it impacts (the march
        // bands the round vs the occupant's published band).
        let target = world
            .spawn(target_bundle(30, 6, worn_suit(0, 0, 1, 0)))
            .id();
        let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));

        // Build the occupancy grid with the target as a HIGH-band occupant in line.
        let mut occupancy = OccupancyGrid::new();
        occupancy.set_occupant(target_at, Some(target));
        occupancy.set_occupant_band(target_at, Some(HeightBand::High));
        let surface = SurfaceGrid::new();
        let cover = CoverLedger::new();
        let mut r = rng();

        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let reports = {
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(8, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };

        assert_eq!(reports.len(), 1, "one round fired");
        let Some(report) = reports.first() else {
            return;
        };
        assert_eq!(
            report.kind,
            ShotKind::Ganger(target),
            "the in-line shot must strike the target ganger",
        );
        assert!(
            report.applied.is_some(),
            "a ganger hit must carry an `AppliedDamage` block",
        );
        // The target's pools changed in the world (the fold mutated in place).
        let hp = world.get::<Hp>(target).map(|h| **h);
        assert!(
            hp.is_some_and(|h| h < 30),
            "the target's Hp must have dropped from 30, got {hp:?}",
        );
    }

    /// AC6 (the miss half) — a fire into empty space (no occupant in the path) yields
    /// only a `no_effect` report: one round fired, no `AppliedDamage` block, and (the
    /// target being absent) nothing is mutated.
    #[test]
    fn fire_into_empty_space_is_a_clean_miss() {
        let tuning = CombatTuning::default();
        let mode = single_mode(0.2, 1);
        let mut world = World::new();
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 2,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 10,
                mode,
                aiming: true,
            },
        );
        let occupancy = OccupancyGrid::new(); // no occupant anywhere
        let surface = SurfaceGrid::new();
        let cover = CoverLedger::new();
        let mut r = rng();
        let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
        let reports = {
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(40, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };
        assert_eq!(reports.len(), 1, "one round fired into empty space");
        let Some(report) = reports.first() else {
            return;
        };
        assert_eq!(report.applied, None, "a clean miss applies no damage");
    }

    /// AC7 — seeded determinism: two same-seed `fire()` runs over identical worlds
    /// produce byte-equal volley reports.
    #[test]
    fn same_seed_reproduces_byte_equal_volley() {
        let tuning = CombatTuning::default();
        let mode = single_mode(0.15, 4);

        let run = || {
            let mut world = World::new();
            let shooter = spawn_shooter(
                &mut world,
                ShooterSpec {
                    x: 2,
                    y: 5,
                    tu: 200,
                    tu_max: 100,
                    ammo: 10,
                    mode,
                    aiming: true,
                },
            );
            let target = world
                .spawn(target_bundle(60, 12, worn_suit(1, 4, 20, 1)))
                .id();
            let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
            let mut occupancy = OccupancyGrid::new();
            occupancy.set_occupant(target_at, Some(target));
            occupancy.set_occupant_band(target_at, Some(HeightBand::High));
            let surface = SurfaceGrid::new();
            let cover = CoverLedger::new();
            let mut r = rng();
            let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
            let (mut shooters, mut targets) = state.get_mut(&mut world);
            fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(8, 5),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &cover,
                },
                &tuning,
                &mut r,
            )
        };

        assert_eq!(
            run(),
            run(),
            "the same battle seed must reproduce the identical volley reports",
        );
    }
}

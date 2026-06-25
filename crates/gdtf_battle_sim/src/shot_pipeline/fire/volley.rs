//! The E4.5 capstone [`fire`] verb — runs the whole firing volley over the two
//! disjoint queries and returns the frozen `Vec<HitReport>`. The validation / charge /
//! clamp / per-round composition steps live in [`compose`](super::compose); the query
//! shapes and arg bundles in [`query`](super::query).

use bevy::prelude::Entity;

use super::{
    compose::{RoundSetup, ShooterReads, TargetGeometry, read_shooter, resolve_round},
    query::{
        BattleGrids, FireOrder, PieceQuery, ShooterQuery, TargetQuery, WeaponQuery, WearsQuery,
        WieldsQuery,
    },
};
use crate::{
    magazine::{FireActor, can_fire, clamp_burst, mode_tu_cost},
    resolve_and_apply::HitReport,
    resolve_coarse::ShotOutcome,
    rng::SimRng,
    tu::spend_tu,
    tuning::CombatTuning,
};

/// The **frozen volley** [`fire`] returns — the per-round [`HitReport`] reports PLUS the
/// matching per-round [`ShotOutcome`] trajectory geometry, in fired order.
///
/// The [`reports`](Volley::reports) are the E3.9 damage / wound verdicts (unchanged from
/// the pre-GTW-290 `Vec<HitReport>` return). The [`shots`](Volley::shots) are the
/// already-computed E2 [`ShotOutcome`] geometry of the SAME rounds — the muzzle /
/// trajectory / impact the presenter draws the GTW-290 muzzle / tracer / impact FX from,
/// exposed (not recomputed) so [`dispatch_fire`](crate::acts::dispatch_fire) can emit one
/// [`ShotFired`](crate::shot_fired::ShotFired) per round. The two vectors are parallel:
/// `reports[i]` and `shots[i]` are the same fired round, so both have the clamped-burst
/// length.
///
/// A transparent value record of the two named result vectors (not itself a wrapped
/// domain scalar). Derives [`PartialEq`] — NOT [`Eq`]: a [`ShotOutcome`] holds `f32`
/// positions, so volley equality is the bit-wise seeded-replay comparison (the same
/// [`BattleSeed`](crate::rng::BattleSeed) reproduces a byte-equal volley, AC7).
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Volley {
    /// The per-round damage / wound reports, in fired order (the pre-GTW-290 result).
    pub reports: Vec<HitReport>,
    /// The per-round [`ShotOutcome`] trajectory geometry, parallel to
    /// [`reports`](Volley::reports) — the GTW-290 FX source.
    pub shots:   Vec<ShotOutcome>,
}

impl Volley {
    /// The **empty** volley — no rounds fired, mutating nothing (the fail-closed result
    /// when [`can_fire`] fails, the shooter is not in the query, or the magazine is
    /// empty).
    pub(crate) const fn empty() -> Self {
        Self {
            reports: Vec::new(),
            shots:   Vec::new(),
        }
    }
}

/// Run the whole firing act and return its **frozen volley** — the E4.5 capstone
/// integrator (`docs/combat/resolution.md` §1 / §1a; the authoritative-model role
/// this crate plays in the model/view split, ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A proper query-based Bevy function — **NO `&mut World`** (AC1): the SHOOTER
/// query ([`ShooterQuery`], `With<Weapon>`) carries the shooter's ganger state plus
/// each GTW-200 weapon-stat component (mutable on
/// [`Tu`](crate::ganger::Tu) + [`Magazine`](crate::magazine::Magazine)); the TARGET
/// query ([`TargetQuery`]) carries the struck ganger's four mutable battle surfaces
/// plus the two read attribute stats — a disjoint mutable set so the two coexist
/// (only [`Luck`](crate::ganger::Luck), read in both, overlaps, and a `&`-vs-`&`
/// overlap is compatible). The act:
///
/// 1. **Validate** ([`can_fire`], the shooter's [`LifeState`](crate::ganger::LifeState)
///    read from the target query): on failure → an EMPTY volley, mutating NOTHING
///    (AC2, fail-closed).
/// 2. **Charge** the full mode TU ONCE via [`spend_tu`] ([`mode_tu_cost`]) — once
///    regardless of round count (AC3).
/// 3. **Clamp** the burst to ammo ([`clamp_burst`]); the
///    [`Magazine`](crate::magazine::Magazine) decrements one round per fired iteration
///    (saturating, AC4).
/// 4. **Per-round loop** ([`resolve_round`](super::compose::resolve_round)): compose
///    every [`ShotInputs`](crate::resolve_coarse::ShotInputs) field, run
///    [`resolve_coarse`](crate::resolve_coarse::resolve_coarse), and fold
///    [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) onto the
///    struck target (got via `targets.get_mut(entity)`) on a
///    [`ShotKind::Ganger`](crate::resolve_coarse::ShotKind::Ganger) — else
///    [`HitReport::no_effect`] (AC5 / AC6). Recoil climbs across the burst
///    (`prior_shots = i`) and RESETS between `fire()` calls.
///
/// Every draw bottoms out in the injected [`SimRng`] (no `thread_rng` / ad-hoc
/// entropy), so the same [`BattleSeed`](crate::rng::BattleSeed) reproduces a
/// byte-equal volley (AC7); no LOS / fog is consulted (the presenter boundary). The
/// aim cell + selected mode ride in `order` ([`FireOrder`]); the world grids in
/// `grids` ([`BattleGrids`]). **Zero pixels** — the reports carry only damage / wound
/// math, and the [`Volley::shots`] geometry rides in sim units (a cubic-voxel
/// [`SimPos`](crate::metric::SimPos) muzzle, a unit-vector trajectory), never a screen
/// coordinate.
///
/// Returns the frozen [`Volley`] — the per-round [`HitReport`] reports PLUS the parallel
/// per-round [`ShotOutcome`] geometry (the GTW-290 FX source, exposed verbatim, not
/// recomputed). Returns an [`empty`](Volley::empty) volley (and mutates nothing) when
/// [`can_fire`] fails, the shooter entity is not in the shooter query, or the magazine is
/// empty. The struck entity not being a queryable target (e.g. it lacks a target
/// component) folds that round to [`HitReport::no_effect`] — never a panic (the round's
/// [`ShotOutcome`] still rides in [`Volley::shots`]).
///
/// Since GTW-323 slice 1 (ADR-0004) the struck location's worn armor is read + worn
/// through the `wears` ([`WearsQuery`]) → `pieces` ([`PieceQuery`]) relationship
/// traversal (`ganger → Wears → the BodyPart-tagged piece`), not a `&mut WornArmor`
/// column; since slice 2 the shooter's weapon stats + [`Magazine`](crate::magazine::Magazine)
/// are read + decremented through the `wields` ([`WieldsQuery`]) → `weapons`
/// ([`WeaponQuery`]) traversal (`ganger → Wields → the weapon entity`), not weapon
/// columns on the [`ShooterQuery`].
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-323 armor + weapon relationships add the disjoint wears/pieces + \
              wields/weapons queries to the fire() signature; bundling them would \
              obscure the query-disjointness the signature documents"
)]
pub fn fire(
    shooter: Entity,
    order: FireOrder,
    shooters: &mut ShooterQuery,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    wields: &WieldsQuery,
    weapons: &mut WeaponQuery,
    mut grids: BattleGrids,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> Volley {
    // (1) Snapshot the shooter's Copy read state up front (the immutable borrows are
    //     released before the mutable re-borrows). A shooter not in the shooter query
    //     (despawned), wielding no weapon, or whose weapon entity is not in the weapon
    //     query fires nothing (fail-closed). The weapon stats come off the related
    //     weapon entity (`ganger → Wields → the weapon entity`, GTW-323 slice 2).
    let Some(ShooterReads {
        snapshot,
        weapon: weapon_entity,
        tu: shooter_tu,
        tu_max: shooter_tu_max,
        aiming: shooter_aiming,
        magazine: magazine_now,
    }) = read_shooter(shooter, shooters, wields, weapons)
    else {
        return Volley::empty();
    };

    // The shooter's own liveness is read from the TARGET query (the shooter is also
    // a ganger), so &LifeState never enters the shooter query (it would clash with
    // the target query's &mut LifeState). A shooter with no target-query components
    // cannot be validated as alive → fail-closed (empty volley, no mutation).
    let Ok((_, _, shooter_life, ..)) = targets.get(shooter) else {
        return Volley::empty();
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
        return Volley::empty();
    }

    // (2) CHARGE the full mode TU ONCE up front (AC3) — debit the shooter's Tu via
    //     the shared mode_tu_cost (the same source can_fire's affordability read).
    let charge = mode_tu_cost(order.mode, &shooter_tu_max, &shooter_aiming, tuning);
    if let Ok((_, mut tu_mut)) = shooters.get_mut(shooter) {
        spend_tu(&mut tu_mut, charge);
    } else {
        // Unreachable after the get() above succeeded, but stay panic-free.
        return Volley::empty();
    }

    // (3) CLAMP the burst to the rounds actually loaded (AC4): min(ModeShots, ammo).
    let rounds = *clamp_burst(order.mode.shots, &magazine_now);

    // (4) The target geometry, composed once (constant across the burst); the
    //     constant-per-burst inputs bundled for the per-round verb. The occupancy grid
    //     supplies the target's published silhouette band (GTW-304/GTW-314) so the aim
    //     lands inside a crouching / prone target's band, not over it.
    let geometry = TargetGeometry::compose(
        order.target_cell,
        order.target_level,
        grids.cover,
        grids.occupancy,
    );
    let setup = RoundSetup {
        snapshot: &snapshot,
        geometry,
        mode: order.mode,
    };

    // (5) PER-ROUND LOOP. Recoil climbs with prior_shots = i (the round index), and
    //     resets between fire() calls because i restarts at 0 every call.
    let mut reports = Vec::with_capacity(usize::from(rounds));
    let mut shots = Vec::with_capacity(usize::from(rounds));
    for i in 0..rounds {
        // resolve_round returns BOTH the report AND the round's already-computed
        // ShotOutcome geometry (the GTW-290 FX source — exposed, not recomputed).
        let (report, outcome) = resolve_round(
            setup,
            crate::cone::PriorShots::new(i),
            &mut grids,
            targets,
            wears,
            pieces,
            tuning,
            rng,
        );

        // Decrement the magazine one round per fired iteration (saturating, AC4) —
        // re-borrow the WEAPON entity mutably (GTW-323 slice 2: the Magazine lives on
        // the weapon now, disjoint from the ganger entities of the other queries).
        if let Ok((.., mut mag_mut)) = weapons.get_mut(weapon_entity) {
            mag_mut.spend_round();
        }

        reports.push(report);
        shots.push(outcome);
    }

    Volley { reports, shots }
}

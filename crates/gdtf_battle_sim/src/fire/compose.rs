//! The per-round composition helpers behind [`fire`](super::fire) — the shooter
//! `Copy`-snapshot, the once-composed target geometry, the constant-per-burst round
//! setup, and the two private verbs ([`read_shooter`] / [`resolve_round`]) that read
//! the shooter off its query and resolve one round of the burst.
//!
//! These are crate-private composition steps (`pub(super)` for [`volley`](super::fire)
//! to call); the public surface is the query shapes ([`query`](super::query)) and
//! [`fire`](super::fire) itself.

use bevy::prelude::Entity;

use super::query::{BattleGrids, ShooterQuery, TargetQuery};
use crate::{
    aim::{Shooter, cone_for, stability_for},
    cover::CoverLedger,
    ganger::{Aiming, Facing, Luck, Position, Shooting, Stance, StanceKind, Tu, TuMax},
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    resolve_and_apply::{HitReport, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotInputs, ShotKind, resolve_coarse},
    rng::SimRng,
    sample_cone::concentration_p,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Kickback, Stable, WeaponDamage,
        WeaponPunch, WeaponShred, WeaponStats,
    },
};

/// The shooter's `Copy` read state, snapshotted **before** the burst loop so the
/// shooter query is only re-borrowed (mutably, for the `Magazine` decrement) one
/// round at a time.
///
/// Reading every `Copy` shooter stat into one local up front releases the immutable
/// borrow of the shooter query, so the loop's `shooters.get_mut(shooter)` (the
/// per-round magazine decrement) does not overlap a live read borrow — the
/// bevy-expert's borrow discipline for the two-query design. Every field is an
/// owned named domain value (no bare primitive); it is an internal call-site
/// snapshot, not a wrapped domain scalar.
pub(super) struct ShooterSnapshot {
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
pub(super) struct TargetGeometry {
    position:   Position,
    stance:     Stance,
    cover_band: Option<crate::cover::HeightBand>,
}

impl TargetGeometry {
    /// Compose the target geometry once from the aim `(cell, level)` and the model
    /// cover ledger (peeked for the target cell's band — never rebuilt).
    pub(super) fn compose(target_cell: Cell, target_level: Level, cover: &CoverLedger) -> Self {
        let at = CellLevel::new(target_cell, target_level);
        Self {
            position:   Position::new(at),
            stance:     Stance::new(StanceKind::Standing),
            cover_band: cover.peek(&at).map(|entry| entry.height_band),
        }
    }
}

/// Read the shooter's `Copy` state off the shooter query into a [`ShooterSnapshot`]
/// plus the [`FireActor`](crate::magazine::FireActor)-shaping economy reads —
/// releasing the query's immutable borrow before [`fire`](super::fire)'s mutable
/// re-borrows.
///
/// Returns `None` when the shooter is not in the query (unarmed / despawned), so
/// [`fire`](super::fire) fails closed. The `(Tu, TuMax, Aiming, Magazine, Luck)` tuple
/// is the economy state [`can_fire`](crate::magazine::can_fire) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) / [`resolve_and_apply`] read.
pub(super) fn read_shooter(
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
pub(super) struct RoundSetup<'a> {
    pub(super) snapshot: &'a ShooterSnapshot,
    pub(super) geometry: TargetGeometry,
    pub(super) mode:     &'a FireModeSpec,
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
pub(super) fn resolve_round(
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

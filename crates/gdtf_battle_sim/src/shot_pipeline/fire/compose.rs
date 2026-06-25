//! The per-round composition helpers behind [`fire`](super::fire) — the shooter
//! `Copy`-snapshot, the once-composed target geometry, the constant-per-burst round
//! setup, and the two private verbs ([`read_shooter`] / [`resolve_round`]) that read
//! the shooter off its query and resolve one round of the burst.
//!
//! These are crate-private composition steps (`pub(super)` for [`volley`](super::fire)
//! to call); the public surface is the query shapes ([`query`](super::query)) and
//! [`fire`](super::fire) itself.

use bevy::prelude::Entity;

use super::query::{
    BattleGrids, PieceQuery, ShooterQuery, TargetQuery, WeaponQuery, WearsQuery, WieldsQuery,
};
use crate::{
    aim::{Shooter, cone_for, stability_for},
    armor::BodyPart,
    cover::CoverLedger,
    ganger::{Aiming, Facing, LifeState, Luck, Position, Shooting, Stance, StanceKind, Tu, TuMax},
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{HitReport, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotInputs, ShotKind, resolve_coarse},
    rng::SimRng,
    sample_cone::concentration_p,
    stability::terrain_brace::terrain_braces,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Kickback, Stable, WeaponDamage,
        WeaponPunch, WeaponShred, WeaponStats,
    },
};

/// Resolve a struck ganger's worn-armor piece **entity** for a struck [`BodyPart`] —
/// the `ganger → Wears → the BodyPart-tagged piece` keyed lookup (GTW-323 / ADR-0004).
///
/// Reads the ganger's [`Wears`](crate::armor::Wears) collection (read-only,
/// `wears.get(ganger)`), iterates its related piece entities, and returns the one
/// tagged with `part` — keyed access (NOT order-dependent), so the looked-up piece is
/// identical regardless of entity storage / spawn order (the determinism property of
/// ADR-0004). Returns `None` when the ganger has no `Wears` collection or no piece
/// tags `part` (folds to bare flesh upstream). The `pieces` query is borrowed
/// immutably here only to read each candidate's [`BodyPart`] tag; the caller re-borrows
/// it mutably to wear the resolved piece.
fn struck_piece_entity(
    ganger: Entity,
    part: BodyPart,
    wears: &WearsQuery,
    pieces: &PieceQuery,
) -> Option<Entity> {
    let worn = wears.get(ganger).ok()?;
    worn.pieces()
        .find(|&piece| pieces.get(piece).is_ok_and(|p| *p.part == part))
}

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
/// The target position is the player's aim `(cell, level)`. The aim **z** comes from
/// the [`cover_band`](TargetGeometry::cover_band): a published band routes the shot
/// through [`target_aim_point`](crate::central_axis::target_aim_point)'s band-midpoint
/// branch, so the central axis lands squarely **inside** the band the §2 clearance
/// test ([`round_clears_occupant`](crate::clearance::round_clears_occupant)) compares
/// against (`docs/combat/resolution.md` §1 aim point; §2 clearance). That band is, in
/// order: the model cover ledger's entry at the target cell (so a deliberately-shot
/// crate aims at its own band midpoint), else the **occupant's published silhouette
/// band** ([`OccupancyGrid::occupant_band`], GTW-304 — a Standing target bands HIGH, a
/// Crouching target MID, a Prone target LOW), else `None`.
///
/// The [`stance`](TargetGeometry::stance) field is the **inert documented fallback**
/// for the `cover_band == None` case ONLY: the locked TARGET query (the disjoint
/// mutable set, AC1) carries no `Stance` to read, so [`StanceKind::Standing`] stands in
/// when no band is published. Every field is an existing named domain value (no bare
/// primitive).
#[derive(Debug, Clone, Copy)]
pub(super) struct TargetGeometry {
    position:   Position,
    stance:     Stance,
    cover_band: Option<crate::cover::HeightBand>,
}

impl TargetGeometry {
    /// Compose the target geometry once from the aim `(cell, level)`, the model cover
    /// ledger, and the occupancy grid (both peeked at the target cell — never rebuilt).
    ///
    /// The aim band is the cover ledger's entry at the target cell, falling back to the
    /// occupant's published silhouette band ([`OccupancyGrid::occupant_band`], GTW-304)
    /// — the SAME band the §2 clearance test reads — so a standing shooter's central
    /// axis lands inside a crouching (MID) / prone (LOW) target's band instead of
    /// sailing over it (`docs/combat/resolution.md` §1 aim point; §2 clearance). With
    /// no band published at all, the aim falls back to the inert
    /// [`stance`](TargetGeometry::stance) = [`StanceKind::Standing`] field.
    pub(super) fn compose(
        target_cell: Cell,
        target_level: Level,
        cover: &CoverLedger,
        occupancy: &OccupancyGrid,
    ) -> Self {
        let at = CellLevel::new(target_cell, target_level);
        // Prefer the cover band (a deliberately-shot crate), else the occupant's
        // published silhouette band (a bare ganger target) — both are the band the §2
        // clearance test compares the round against.
        let aim_band = cover
            .peek(&at)
            .map(|entry| entry.height_band)
            .or_else(|| occupancy.occupant_band(&at));
        Self {
            position:   Position::new(at),
            stance:     Stance::new(StanceKind::Standing),
            cover_band: aim_band,
        }
    }
}

/// The shooter read result — the `Copy` [`ShooterSnapshot`], the wielded
/// [`weapon`](ShooterReads::weapon) entity (whose [`Magazine`] the burst decrements),
/// and the [`FireActor`](crate::magazine::FireActor)-shaping economy reads.
///
/// Bundles the values [`read_shooter`] hands back so [`fire`](super::fire) can validate
/// the act (the `(Tu, TuMax, Aiming, Magazine)` economy), charge the up-front TU, and —
/// across the burst loop — re-borrow the weapon entity to spend a round per fired
/// iteration. Every field is an owned named domain value or a Bevy [`Entity`] handle
/// (framework plumbing); the bundle itself is a transparent call-site record, not a
/// wrapped domain scalar.
pub(super) struct ShooterReads {
    /// The shooter's `Copy` ganger-state + weapon-stat snapshot.
    pub(super) snapshot: ShooterSnapshot,
    /// The wielded weapon entity — the burst re-borrows its [`Magazine`] per round.
    pub(super) weapon:   Entity,
    /// The shooter's current TU pool ([`can_fire`](crate::magazine::can_fire) reads it).
    pub(super) tu:       Tu,
    /// The shooter's TU ceiling ([`mode_tu_cost`](crate::magazine::mode_tu_cost) reads it).
    pub(super) tu_max:   TuMax,
    /// Whether the shooter is aiming (the ×1.5 TU premium toggle).
    pub(super) aiming:   Aiming,
    /// The weapon's ammo state, snapshotted for the affordability / burst-clamp reads.
    pub(super) magazine: Magazine,
}

/// Read the shooter's `Copy` state into a [`ShooterSnapshot`] — its ganger state off the
/// [`ShooterQuery`] and its GTW-200 weapon stats off the related **weapon entity**
/// (`ganger → Wields → the weapon entity`, GTW-323 slice 2) — plus the
/// [`FireActor`](crate::magazine::FireActor)-shaping economy reads, releasing the query
/// borrows before [`fire`](super::fire)'s mutable re-borrows.
///
/// Returns `None` when the shooter is not in the shooter query (despawned), wields no
/// weapon (no [`Wields`](crate::weapon::Wields) collection / it is empty), or the
/// resolved weapon entity is not in the weapon query — so [`fire`](super::fire) fails
/// closed in every unarmed/missing case. The economy reads
/// `(Tu, TuMax, Aiming, Magazine)` are what [`can_fire`](crate::magazine::can_fire) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) / [`resolve_and_apply`] consume; the
/// returned [`weapon`](ShooterReads::weapon) entity is the one the burst loop re-borrows
/// to decrement the [`Magazine`] per fired round.
pub(super) fn read_shooter(
    shooter: Entity,
    shooters: &ShooterQuery,
    wields: &WieldsQuery,
    weapons: &WeaponQuery,
) -> Option<ShooterReads> {
    let ((position, facing, stance, aiming, shooting, luck, tu_max), tu) =
        shooters.get(shooter).ok()?;
    // Resolve `ganger → Wields → the weapon entity` (GTW-323 slice 2), then read the
    // GTW-200 weapon stats + magazine off that weapon entity (a different entity than
    // the ganger, so the borrow is disjoint).
    let weapon = wields.get(shooter).ok()?.weapon()?;
    let (
        base_spread,
        accuracy,
        kickback,
        fatal_bias,
        damage,
        punch,
        shred,
        damage_type,
        stable,
        magazine,
    ) = weapons.get(weapon).ok()?;
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
    Some(ShooterReads {
        snapshot,
        weapon,
        tu: *tu,
        tu_max: *tu_max,
        aiming: *aiming,
        magazine: *magazine,
    })
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
/// — a struck entity that is not a queryable target folds to [`HitReport::no_effect`],
/// never a panic); a [`ShotKind::Cover`] / [`ShotKind::Slab`] / [`ShotKind::Ground`]
/// outcome ALSO folds through [`resolve_and_apply`] (the cover/slab arms spend their
/// ledger HP, the ground arm — GTW-366 — records the round's `weapon_damage` accrual in
/// the report); only a clean [`ShotKind::Miss`] short-circuits to
/// [`HitReport::no_effect`]. Every draw is from the injected [`SimRng`].
///
/// Returns BOTH the frozen [`HitReport`] **and** the round's
/// [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — the already-computed E2
/// trajectory geometry [`fire`](super::fire) collects so
/// [`dispatch_fire`](crate::acts::dispatch_fire) can emit a per-round
/// [`ShotFired`](crate::shot_fired::ShotFired) (GTW-290). The outcome is returned
/// verbatim, NOT recomputed — the fold below already consumes it.
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-323 armor-relationship adds the disjoint wears/pieces queries to \
              the per-round verb; bundling them would obscure the query-disjointness"
)]
pub(super) fn resolve_round(
    setup: RoundSetup,
    prior_shots: crate::cone::PriorShots,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> (HitReport, crate::resolve_coarse::ShotOutcome) {
    let snapshot = setup.snapshot;
    let geometry = setup.geometry;
    let shooter_view = snapshot.shooter_view();
    // GTW-392: compute the terrain brace ONCE per round from the live grids. The
    // snapshot's position + stance were read before the burst loop — correct by design
    // (stance cannot change mid-burst; revocation is a cross-action property). Both
    // cone_for and stability_for receive the same terrain_braced value so cone width +
    // recoil damping are consistent within the burst round.
    let terrain_braced = terrain_braces(
        snapshot.position,
        *snapshot.stance,
        grids.brace_cells,
        grids.surface,
    );
    let cone = cone_for(
        &shooter_view,
        snapshot.weapon_stats(),
        setup.mode,
        prior_shots,
        grids.cover,
        terrain_braced,
        tuning,
    );
    let (_cone_mult, recoil_growth) = stability_for(
        &shooter_view,
        snapshot.stable,
        terrain_braced,
        grids.cover,
        tuning,
    );
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

    // GTW-317 dead-occupant skip: the march passes THROUGH corpses (a `Dead` ganger)
    // and continues to the next blocker, while a live (incl. `Downed`) occupant still
    // stops the round. The predicate reads each candidate occupant's CURRENT
    // `LifeState` straight off the TARGET query, so a burst round that kills the front
    // target writes `Dead` to its `LifeState` (via `resolve_and_apply` below) BEFORE
    // the next round runs — making the next round pass through the fresh corpse. The
    // immutable borrow this closure holds on `targets` ends when `resolve_coarse`
    // returns (NLL), so the later `targets.get_mut(struck)` does not conflict.
    let is_dead = |e: Entity| {
        // The TargetQuery row carries `&LifeState` as its third item; an entity not in
        // the query (e.g. cover, the surface) is never a corpse.
        targets
            .get(e)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };
    let outcome = resolve_coarse(
        &shot,
        grids.occupancy,
        grids.surface,
        grids.cover,
        tuning,
        rng,
        is_dead,
    );

    let report = match outcome.kind {
        ShotKind::Ganger(struck) => fold_ganger_round(
            &outcome, struck, snapshot, grids, targets, wears, pieces, tuning, rng,
        ),
        // GTW-364: a round that strikes COVER folds through the SAME resolve_and_apply,
        // which reuses the ganger damage formula against the cover's own armor, spends
        // the ledger's HP via deplete_cover, and records a destroyed (cell, level) in
        // the report (bridged to a CoverDestroyed message by dispatch_fire). There is no
        // struck ganger (`None` target); the cover ledger is reborrowed `&mut` here (its
        // earlier `&` reborrow by resolve_coarse / cone_for / stability_for has ended).
        // GTW-365: a round that strikes a SLAB takes the same path — it spends the SLAB
        // ledger's HP instead (the StruckSurfaces bundle carries both; the fold's
        // ShotKind selects which one is touched).
        // GTW-366: a round that strikes the GROUND ALSO folds through resolve_and_apply —
        // its Ground arm records the round's weapon_damage in the report's `ground_accrued`
        // (bridged to a GroundAccrued message by dispatch_fire). It touches NEITHER ledger,
        // but routing it through the fold is the production seam the accrual lives on.
        ShotKind::Cover(_) | ShotKind::Slab(_) | ShotKind::Ground(_) => resolve_and_apply(
            &outcome,
            snapshot.weapon_stats(),
            snapshot.luck,
            None,
            Entity::PLACEHOLDER,
            StruckSurfaces {
                cover: grids.cover,
                slab:  grids.slab,
            },
            tuning,
            rng,
        ),
        // A clean miss strikes nothing — no effect.
        ShotKind::Miss => HitReport::no_effect(outcome.kind),
    };
    // Return the resolved report PLUS the already-computed outcome geometry (verbatim,
    // not recomputed) so the volley can surface a per-round ShotFired (GTW-290).
    (report, outcome)
}

/// Fold a [`ShotKind::Ganger`] round onto the struck target — the wound arm of
/// [`resolve_round`], split out (GTW-365) so the per-round verb stays under clippy's
/// line cap once the slab arm joined the cover arm.
///
/// GTW-323 / ADR-0004: resolves the struck location's worn piece ENTITY via
/// `ganger → Wears → the BodyPart-tagged piece`, reads its stats + wears its
/// `&mut ArmorIntegrity` through the fold. The lookup keys on the §4 struck part (carried
/// on `outcome`); a missing part / piece folds to bare flesh (`StruckPiece == None`). The
/// `wears` / `pieces` queries are disjoint from `targets`, so they coexist with the
/// `targets.get_mut(struck)`. A struck entity that is not a queryable target folds to
/// [`HitReport::no_effect`] — never a panic. The [`StruckSurfaces`] bundle is threaded so
/// the SAME [`resolve_and_apply`] signature serves both arms (a ganger hit touches
/// neither ledger).
#[expect(
    clippy::too_many_arguments,
    reason = "the ganger fold needs the outcome / struck entity / snapshot / grids plus \
              the disjoint wears+pieces queries + tuning + rng; bundling the queries would \
              obscure the GTW-323 disjointness the ParamSet-free coexistence relies on"
)]
fn fold_ganger_round(
    outcome: &crate::resolve_coarse::ShotOutcome,
    struck: Entity,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> HitReport {
    let struck_piece_view = outcome
        .body_part
        .and_then(|part| struck_piece_entity(struck, part, wears, pieces))
        .and_then(|piece_entity| {
            pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
        });

    match targets.get_mut(struck) {
        Ok((mut hp, mut wounds, mut life, mut inflicted, toughness, target_luck)) => {
            resolve_and_apply(
                outcome,
                snapshot.weapon_stats(),
                snapshot.luck,
                Some(TargetGanger {
                    hp:        &mut hp,
                    wounds:    &mut wounds,
                    life:      &mut life,
                    piece:     struck_piece_view,
                    inflicted: &mut inflicted,
                    toughness: *toughness,
                    luck:      *target_luck,
                }),
                struck,
                StruckSurfaces {
                    cover: grids.cover,
                    slab:  grids.slab,
                },
                tuning,
                rng,
            )
        }
        Err(_) => HitReport::no_effect(outcome.kind),
    }
}

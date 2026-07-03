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
    BattleGrids, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery, WeaponQuery,
    WearsQuery, WieldsQuery,
};
use crate::{
    aim::{Shooter, cone_for, stability_for},
    armor::BodyPart,
    cover::CoverLedger,
    ganger::{
        Aiming, Facing, LifeState, Luck, Position, Shooting, Stance, StanceKind, Suppressed, Tu,
        TuMax,
    },
    injuries::{HandsAvailable, InflictedInjuries, InjuryRegistry, InjuryTables},
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{HitReport, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotInputs, ShotKind, resolve_coarse},
    rng::{InjuryRng, SeverityRng, ShotRng},
    sample_cone::concentration_p,
    stability::{EmplacementStability, terrain_brace::terrain_braces},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Handedness, Kickback, Stable,
        WeaponBraceBonus, WeaponDamage, WeaponPunch, WeaponShred, WeaponStats,
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
    // GTW-549: the weapon's optional per-item WeaponBraceBonus attachment, snapshotted so
    // cone_for / stability_for fold the additive graduated brace bonus into the burst's
    // dispersion cone. `None` = no brace attachment = the zero-identity brace term.
    // `WeaponBraceBonus` is `Copy`, so the snapshot owns it and `weapon_stats` hands out an
    // `Option<&WeaponBraceBonus>` borrow. SUPERSEDES the GTW-542 Scoped / sight_bonus fields.
    brace_bonus: Option<WeaponBraceBonus>,
    // GTW-526: the shooter's Suppressed state, snapshotted so cone_for / stability_for
    // widen the burst's dispersion cone while the shooter is pinned. `None` = un-suppressed
    // = the zero-identity suppression term. `Suppressed` is `Copy`, so the snapshot owns it
    // and `shooter_view` hands out an `Option<&Suppressed>` borrow.
    suppressed:  Option<Suppressed>,
    // GTW-543: whether the shooter's RESOLVED ranged weapon is a MountedWeapon — `true` when the
    // ganger is MANNING an emplacement and firing its bolted-down gun. Feeds the emplacement
    // stability seam (cone_for / the recoil-recompute stability_for), steadying the deliberately
    // inaccurate mount. `false` (an un-mounted / normal shot) resolves the zero-identity
    // EmplacementStability, so the shot is byte-identical to before the seam engaged.
    mounted:     bool,
    // GTW-544: the weapon's optional DamageProfile-over-time (`DotProfile`), snapshotted so
    // `weapon_stats` hands out its `Option<&DotProfile>` borrow to the fold — a penetrating hit
    // from a DOT weapon attaches a `Dot` on the struck ganger. `None` = a non-DOT weapon (no
    // attach, byte-identical). `DotProfile` is `Copy`, so the snapshot owns it.
    dot:         Option<crate::weapon::DotProfile>,
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
            // GTW-549: borrow the snapshotted per-item brace bonus (None = no brace attachment).
            brace_bonus: self.brace_bonus.as_ref(),
            // GTW-544: borrow the snapshotted DOT profile (None = a non-DOT weapon).
            dot:         self.dot.as_ref(),
        }
    }

    /// Assemble a transient [`Shooter`] borrow-view over this snapshot's ganger state
    /// — the read-shape [`cone_for`] / [`stability_for`] take.
    const fn shooter_view(&self) -> Shooter<'_> {
        Shooter {
            stance:     &self.stance,
            aiming:     &self.aiming,
            position:   &self.position,
            facing:     &self.facing,
            // GTW-526: borrow the snapshotted Suppressed state (None = un-suppressed).
            suppressed: self.suppressed.as_ref(),
        }
    }

    /// The GTW-543 emplacement stability term for this shot — the tunable
    /// [`EmplacementStabilityBonus`](crate::tuning::EmplacementStabilityBonus) when the shooter's
    /// resolved ranged weapon is a [`MountedWeapon`](crate::weapon::MountedWeapon) (the ganger is
    /// MANNING an emplacement), else [`EmplacementStability::none`] (the zero identity).
    ///
    /// Resolved ONCE here so the two burst-round reads — [`cone_for`] (cone width) and the
    /// recoil-recompute [`stability_for`] — feed the SAME emplacement term, keeping the cone and
    /// its recoil damping consistent within a round (the GTW-542 `sight_stability` shared-term
    /// precedent). An un-mounted shot resolves the zero identity, so its stability score — and
    /// thus its cone — is byte-identical to before the seam engaged (the pure-additive property).
    fn emplacement_stability(&self, tuning: &CombatTuning) -> EmplacementStability {
        if self.mounted {
            EmplacementStability::new(*tuning.cone_stability.emplacement_stability_bonus)
        } else {
            EmplacementStability::none()
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
    pub(super) snapshot:        ShooterSnapshot,
    /// The wielded weapon entity — the burst re-borrows its [`Magazine`] per round.
    pub(super) weapon:          Entity,
    /// The shooter's current TU pool ([`can_fire`](crate::magazine::can_fire) reads it).
    pub(super) tu:              Tu,
    /// The shooter's TU ceiling ([`mode_tu_cost`](crate::magazine::mode_tu_cost) reads it).
    pub(super) tu_max:          TuMax,
    /// Whether the shooter is aiming (the ×1.5 TU premium toggle).
    pub(super) aiming:          Aiming,
    /// The weapon's ammo state, snapshotted for the affordability / burst-clamp reads.
    pub(super) magazine:        Magazine,
    /// The wielded weapon's [`Handedness`] (GTW-443) — fed to the [`FireActor`](crate::magazine::FireActor)
    /// hand-count gate.
    pub(super) handedness:      Handedness,
    /// The shooter's [`HandsAvailable`] (GTW-443), FOLDED from its injury ledger (an
    /// absent ledger = the uninjured two-hands default) — fed to the
    /// [`FireActor`](crate::magazine::FireActor) hand-count gate.
    pub(super) hands_available: HandsAvailable,
}

/// Read the shooter's `Copy` state into a [`ShooterSnapshot`] — its ganger state off the
/// [`ShooterQuery`] and its GTW-200 weapon stats off the related **weapon entity**
/// (`ganger → Wields → the weapon entity`, GTW-323 slice 2) — plus the
/// [`FireActor`](crate::magazine::FireActor)-shaping economy reads, releasing the query
/// borrows before [`fire`](super::fire)'s mutable re-borrows.
///
/// Returns `None` when the shooter is not in the shooter query (despawned), wields no
/// RANGED weapon (no [`Wields`](crate::weapon::Wields) collection / it holds only a
/// melee weapon), or the resolved ranged weapon entity is not in the weapon query — so
/// [`fire`](super::fire) fails closed in every unarmed/missing case. The economy reads
/// `(Tu, TuMax, Aiming, Magazine)` are what [`can_fire`](crate::magazine::can_fire) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) / [`resolve_and_apply`] consume; the
/// returned [`weapon`](ShooterReads::weapon) entity is the one the burst loop re-borrows
/// to decrement the [`Magazine`] per fired round.
///
/// GTW-505 C5: the ranged weapon is resolved via
/// [`Wields::ranged_weapon`](crate::weapon::Wields::ranged_weapon) over the `melee`
/// [`MeleeQuery`] probe — EXCLUDING the melee weapon the same ganger also wields — so
/// relating a melee weapon never regresses ranged firing.
///
/// GTW-543: the resolution PREFERS a [`MountedWeapon`](crate::weapon::MountedWeapon)-marked
/// wielded entity (the `mounted` [`MountedQuery`] probe) when present —
/// `wields.mounted_weapon(..).or_else(|| wields.ranged_weapon(..))` — so a ganger MANNING an
/// emplacement fires its bolted-down gun, and reverts to its own carried weapon when it exits
/// (the mount edge despawned). A ganger with no mounted weapon (the common case) falls straight
/// through to the GTW-505 ranged resolution, byte-identical to before. The
/// [`mounted`](ShooterSnapshot::mounted) flag on the snapshot records which was resolved so the
/// emplacement stability seam engages for a mounted shot only.
pub(super) fn read_shooter(
    shooter: Entity,
    shooters: &ShooterQuery,
    wields: &WieldsQuery,
    weapons: &WeaponQuery,
    melee: &MeleeQuery,
    mounted: &MountedQuery,
) -> Option<ShooterReads> {
    let ((position, facing, stance, aiming, shooting, luck, tu_max, suppressed), injuries, tu) =
        shooters.get(shooter).ok()?;
    // GTW-436: read the shooter's Luck through the gate-enforced effective accessor over
    // its injury ledger (an absent ledger = the zero-delta identity), so a `Modify(Luck)`
    // injury shifts the wounds it deals. This is the SINGLE direct-read path for Luck.
    let effective_shooter_luck = match injuries {
        Some(ledger) => crate::ganger::effective_luck(*luck, ledger),
        None => *luck,
    };
    // GTW-443: fold the shooter's available hand count from its injury ledger (an absent
    // ledger = the uninjured two-hands default) for the FireActor hand-count gate.
    let hands_available =
        injuries.map_or_else(HandsAvailable::default, InflictedInjuries::hands_available);
    // Resolve `ganger → Wields → the RANGED weapon entity` (GTW-323 slice 2 / GTW-505
    // C5), then read the GTW-200 weapon stats + magazine off that weapon entity (a
    // different entity than the ganger, so the borrow is disjoint). `ranged_weapon`
    // EXCLUDES the melee weapon the ganger also wields (the `melee` marker probe) so the
    // melee entity is never mistaken for the gun. GTW-543: PREFER a MountedWeapon-marked
    // wielded entity (the ganger is manning an emplacement) over its own carried gun; a ganger
    // with no mounted weapon falls straight through to the ranged resolution.
    let wielded = wields.get(shooter).ok()?;
    let mounted_weapon = wielded.mounted_weapon(|entity| mounted.get(entity).is_ok());
    let is_mounted = mounted_weapon.is_some();
    let weapon =
        mounted_weapon.or_else(|| wielded.ranged_weapon(|entity| melee.get(entity).is_ok()))?;
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
        brace_bonus,
        handedness,
        magazine,
        dot,
    ) = weapons.get(weapon).ok()?;
    let snapshot = ShooterSnapshot {
        position:    *position,
        facing:      *facing,
        stance:      *stance,
        aiming:      *aiming,
        shooting:    *shooting,
        luck:        effective_shooter_luck,
        base_spread: *base_spread,
        accuracy:    *accuracy,
        kickback:    *kickback,
        fatal_bias:  *fatal_bias,
        damage:      *damage,
        punch:       *punch,
        shred:       *shred,
        damage_type: *damage_type,
        stable:      *stable,
        // GTW-549: copy the weapon's per-item WeaponBraceBonus attachment into the snapshot
        // (None = no brace attachment = zero-identity brace term).
        brace_bonus: brace_bonus.copied(),
        // GTW-526: copy the shooter's Suppressed state into the snapshot (None =
        // un-suppressed = zero-identity suppression term).
        suppressed:  suppressed.copied(),
        // GTW-543: whether the resolved ranged weapon is the emplacement's mounted gun (the
        // ganger is manning it) — engages the emplacement stability seam for this shot.
        mounted:     is_mounted,
        // GTW-544: copy the resolved weapon's DOT profile into the snapshot (None = a non-DOT
        // weapon = no attach). Rides through `weapon_stats` to the fold's DOT-attach decision.
        dot:         dot.copied(),
    };
    Some(ShooterReads {
        snapshot,
        weapon,
        tu: *tu,
        tu_max: *tu_max,
        aiming: *aiming,
        magazine: *magazine,
        handedness: *handedness,
        hands_available,
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
/// [`HitReport::no_effect`]. Shot draws come from the injected [`ShotRng`](crate::rng::ShotRng);
/// severity draws from the injected [`SeverityRng`](crate::rng::SeverityRng).
///
/// Returns the frozen primary [`HitReport`], the `AoE` **splash** reports (GTW-541 —
/// EMPTY for a [`HitType::Single`](crate::weapon::HitType::Single) round, so the
/// single-target path is byte-identical), **and** the round's
/// [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — the already-computed E2
/// trajectory geometry [`fire`](super::fire) collects so
/// [`dispatch_fire`](crate::acts::dispatch_fire) can emit a per-round
/// [`ShotFired`](crate::shot_fired::ShotFired) (GTW-290). The outcome is returned
/// verbatim, NOT recomputed — the fold below already consumes it.
///
/// GTW-541 (`AoE` CORE of GTW-41): after the primary impact fold, if the fired mode's
/// [`HitType`](crate::weapon::HitType) is not
/// [`Single`](crate::weapon::HitType::Single), the template's affected cells are
/// enumerated ([`aoe_affected`](crate::aoe::aoe_affected)) and EACH occupant (skipping
/// the already-folded direct target and non-ganger cells) is routed through the EXISTING
/// [`resolve_and_apply`] damage path EXACTLY ONCE, faction-blind (friendly fire hits all
/// — `docs/combat/resolution.md` §2). The affected cells are resolved in the resolver's
/// canonical sorted order, so the seeded RNG stream is deterministic. A `Single` round
/// runs NEITHER the resolver nor any extra draw — the identity property.
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-323 armor-relationship adds the disjoint wears/pieces queries to \
              the per-round verb, and GTW-438 adds the injury-roll inputs (InjuryTables + \
              InjuryRegistry reads + the &mut InjuryRng draw stream); bundling them would \
              obscure the query-disjointness + the distinct RNG streams the signature \
              documents"
)]
pub(super) fn resolve_round(
    setup: RoundSetup,
    prior_shots: crate::cone::PriorShots,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
    // GTW-438: the injury-roll inputs, threaded down to the ganger fold (the cover /
    // slab / ground arms ignore them — a structural hit rolls no injury).
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> (
    HitReport,
    Vec<HitReport>,
    crate::resolve_coarse::ShotOutcome,
) {
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
    // GTW-543: resolve the emplacement stability term ONCE (mounted → the tunable bonus, else the
    // zero identity) so cone_for AND the recoil-recompute stability_for below feed the SAME term.
    let emplacement = snapshot.emplacement_stability(tuning);
    let cone = cone_for(
        &shooter_view,
        snapshot.weapon_stats(),
        setup.mode,
        prior_shots,
        grids.cover,
        terrain_braced,
        emplacement,
        tuning,
    );
    // GTW-549: resolve the SAME per-item brace term cone_for used above (off the snapshot's
    // WeaponBraceBonus attachment) so this recoil-growth-only recompute stays consistent with
    // the cone width. `None` (no brace attachment) resolves the zero identity.
    let brace_bonus = snapshot.brace_bonus.unwrap_or_else(WeaponBraceBonus::none);
    let (_cone_mult, recoil_growth) = stability_for(
        &shooter_view,
        snapshot.stable,
        terrain_braced,
        brace_bonus,
        emplacement,
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
        shot_rng,
        is_dead,
    );

    // Fold the PRIMARY impact — the direct-hit report (ganger / cover / slab / ground /
    // miss), extracted to keep this per-round verb under clippy's line cap once the
    // GTW-541 splash pass joined it.
    let report = resolve_primary_report(
        &outcome,
        snapshot,
        grids,
        targets,
        wears,
        pieces,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    );

    // GTW-541 (`AoE` CORE): if the fired mode carries a non-Single HitType, splash the
    // template's other affected cells. `Single` short-circuits (empty splash, no
    // resolver call, no extra draw) so the single-target path is byte-identical.
    let splash = apply_aoe_splash(
        &outcome,
        setup.mode.hit_type,
        snapshot.position,
        &report,
        snapshot,
        grids,
        targets,
        wears,
        pieces,
        tuning,
        shot_rng,
        severity_rng,
        tables,
        registry,
        injury_rng,
    );

    // Return the resolved primary report, the `AoE` splash reports (empty for Single),
    // PLUS the already-computed outcome geometry (verbatim, not recomputed) so the
    // volley can surface a per-round ShotFired (GTW-290).
    (report, splash, outcome)
}

/// Fold the round's PRIMARY (direct-impact) outcome into its [`HitReport`] — the
/// ganger / cover / slab / ground / miss dispatch [`resolve_round`] ran inline before
/// GTW-541 (extracted so the per-round verb stays under clippy's line cap once the splash
/// pass joined it). No behavior change — the same match, verbatim.
///
/// - [`ShotKind::Ganger`] → [`fold_ganger_round`] (the wound arm; the ONE severity +
///   injury draw).
/// - [`ShotKind::Cover`] / [`ShotKind::Slab`] / [`ShotKind::Ground`] → the shared
///   [`resolve_and_apply`] structural path (GTW-364/365/366): a cover / slab hit spends its
///   ledger HP (RNG-free), a ground hit records the accrual — the `None` target /
///   [`Entity::PLACEHOLDER`] short-circuits the wound path (no severity / injury draw).
/// - [`ShotKind::Miss`] → a no-effect report (no draw, no mutation).
#[expect(
    clippy::too_many_arguments,
    reason = "this is the exact irreducible fold set resolve_round passed inline before \
              GTW-541 (outcome / snapshot / grids + the disjoint wears/pieces queries + \
              tuning + the severity/injury RNG streams + the injury tables/registry); \
              bundling the queries would obscure the GTW-323 disjointness the ParamSet-free \
              coexistence relies on — the same reason fold_ganger_round documents"
)]
fn resolve_primary_report(
    outcome: &crate::resolve_coarse::ShotOutcome,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    match outcome.kind {
        ShotKind::Ganger(struck) => fold_ganger_round(
            outcome,
            struck,
            snapshot,
            grids,
            targets,
            wears,
            pieces,
            tuning,
            severity_rng,
            tables,
            registry,
            injury_rng,
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
        // Cover/Slab/Ground arms are RNG-free (no severity draw on a structural hit) so
        // we pass severity_rng but it will not advance the cursor for these arms.
        ShotKind::Cover(_) | ShotKind::Slab(_) | ShotKind::Ground(_) => resolve_and_apply(
            outcome,
            snapshot.weapon_stats(),
            snapshot.luck,
            None,
            Entity::PLACEHOLDER,
            StruckSurfaces {
                cover: grids.cover,
                slab:  grids.slab,
            },
            tuning,
            severity_rng,
            // GTW-438: a structural (cover/slab/ground) hit rolls NO injury — the
            // tables/registry are unread and the InjuryRng cursor never advances for
            // these arms (the `None` target short-circuits the wound path).
            tables,
            registry,
            injury_rng,
        ),
        // A clean miss strikes nothing — no effect.
        ShotKind::Miss => HitReport::no_effect(outcome.kind),
    }
}

/// Splash a non-[`Single`](crate::weapon::HitType::Single) round's `AoE` template onto the
/// OTHER occupants the shape covers — the GTW-541 (`AoE` CORE of GTW-41) resolver-to-damage
/// integration.
///
/// Returns an EMPTY `Vec` for a [`HitType::Single`](crate::weapon::HitType::Single) round
/// WITHOUT calling the resolver or taking any RNG draw — so the single-target path is
/// byte-identical (the GTW-541 identity property). Otherwise it enumerates the affected
/// `(cell, level)` set ([`aoe_affected`](crate::aoe::aoe_affected), from the primary
/// impact cell, the shooter origin, and the mode's [`HitType`](crate::weapon::HitType))
/// and, in that CANONICAL sorted order (so the seeded RNG stream is deterministic), routes
/// each cell's occupant through the EXISTING [`resolve_and_apply`] damage path EXACTLY
/// ONCE — reusing [`fold_ganger_round`] verbatim (no duplicated damage math).
///
/// Faction-blind: the splash strikes EVERY occupant it finds, including the shooter's own
/// gang if the geometry covers them (`docs/combat/resolution.md` §2 — "any other actor in
/// the path — including your own gang — true friendly fire"; grenades do not discriminate).
/// The DIRECT-impact target already folded above (`primary`) is skipped so it is never
/// double-hit. A non-ganger occupancy slot (empty / cover only) contributes nothing.
///
/// Each splashed ganger takes one [`ShotRng`](crate::rng::ShotRng) draw (the §4 body-part
/// roll — the splash has no march-computed part) plus the fold's one
/// [`SeverityRng`](crate::rng::SeverityRng) + one [`InjuryRng`](crate::rng::InjuryRng) draw,
/// EXACTLY the direct-target cost — so the streams stay content-independent and stable.
#[expect(
    clippy::too_many_arguments,
    reason = "the splash pass needs the primary outcome / hit-type / shooter origin / the \
              already-folded primary report (to skip the direct target) / the shooter \
              snapshot / grids plus the disjoint wears+pieces queries + tuning + the three \
              distinct RNG streams (shot / severity / injury) + the injury tables/registry; \
              this is the same irreducible set fold_ganger_round documents, plus the \
              `AoE`-specific outcome/hit-type/origin/primary inputs"
)]
fn apply_aoe_splash(
    outcome: &crate::resolve_coarse::ShotOutcome,
    hit_type: crate::weapon::HitType,
    shooter_position: Position,
    primary: &HitReport,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> Vec<HitReport> {
    // The identity short-circuit: a Single round splashes nothing and takes NO draw.
    if matches!(hit_type, crate::weapon::HitType::Single) {
        return Vec::new();
    }

    // The DIRECT-impact target already folded (skip it so it is never double-hit).
    let primary_struck = match primary.kind {
        ShotKind::Ganger(e) => Some(e),
        _ => None,
    };

    // The impact + shooter cells the resolver keys the template off (the storey is the
    // impact's own — the 2D-on-level ruling). `Position` derefs to `CellLevel`.
    let impact = CellLevel::new(outcome.cell, outcome.level);
    let shooter_cell: CellLevel = *shooter_position;

    let affected = crate::aoe::aoe_affected(impact, hit_type, shooter_cell);
    let mut reports = Vec::new();
    for cell in affected {
        // Read the occupant — a faction-blind cell peek (friendly fire hits all).
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue; // empty / cover-only cell — nothing to strike
        };
        if Some(occupant) == primary_struck {
            continue; // the direct target already took its hit
        }
        // Roll the §4 body part for the splashed ganger (its ONE ShotRng draw — the
        // splash has no march-computed part), then synthesize a Ganger outcome AT the
        // splashed cell and fold it through the SAME per-round ganger path.
        let part = crate::hit_location::roll_body_part(&tuning.body_part_weights, shot_rng.rng());
        let (splash_cell, splash_level) = (cell.cell(), outcome.level);
        let splash_outcome = crate::resolve_coarse::ShotOutcome {
            kind:       ShotKind::Ganger(occupant),
            cell:       splash_cell,
            level:      splash_level,
            body_part:  Some(part),
            band:       outcome.band,
            muzzle:     outcome.muzzle,
            trajectory: outcome.trajectory,
        };
        let report = fold_ganger_round(
            &splash_outcome,
            occupant,
            snapshot,
            grids,
            targets,
            wears,
            pieces,
            tuning,
            severity_rng,
            tables,
            registry,
            injury_rng,
        );
        reports.push(report);
    }
    reports
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
              the disjoint wears+pieces queries + tuning + severity-rng, and GTW-438 adds \
              the injury-roll inputs (InjuryTables + InjuryRegistry + the &mut InjuryRng \
              draw stream); bundling the queries would obscure the GTW-323 disjointness \
              the ParamSet-free coexistence relies on"
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
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
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
        Ok((mut hp, mut wounds, mut life, mut inflicted, toughness, target_luck, injuries)) => {
            // GTW-436: route the defender's Toughness + Luck through the gate-enforced
            // effective accessors over its injury ledger (an absent ledger = the
            // zero-delta identity), so a `Modify(Toughness)` / `Modify(Luck)` injury
            // shifts the §6 severity roll in step with the derived stats. This is the
            // SINGLE direct-read path for the defender's Toughness / Luck.
            let (effective_toughness, effective_luck) = match injuries {
                Some(ledger) => (
                    crate::ganger::effective_toughness(*toughness, ledger),
                    crate::ganger::effective_luck(*target_luck, ledger),
                ),
                None => (*toughness, *target_luck),
            };
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
                    toughness: effective_toughness,
                    luck:      effective_luck,
                }),
                struck,
                StruckSurfaces {
                    cover: grids.cover,
                    slab:  grids.slab,
                },
                tuning,
                severity_rng,
                tables,
                registry,
                injury_rng,
            )
        }
        Err(_) => HitReport::no_effect(outcome.kind),
    }
}

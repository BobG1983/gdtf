//! The per-round driver — compose the once-per-burst target geometry and each
//! round's [`ShotInputs`], run E2 [`resolve_coarse`], and delegate the impact
//! folds to [`fold`](super::fold) / [`splash`](super::splash).

use bevy::prelude::Entity;

use super::{
    super::query::{BattleGrids, PieceQuery, TargetQuery, WearsQuery},
    fold::resolve_primary_report,
    snapshot::ShooterSnapshot,
    splash::apply_aoe_splash,
};
use crate::{
    aim::{cone_for, stability_for},
    cover::CoverLedger,
    effects::attachments::WeaponBraceBonus,
    ganger::{LifeState, Position, Stance, StanceKind},
    injuries::{InjuryRegistry, InjuryTables},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::HitReport,
    resolve_coarse::{ShotInputs, resolve_coarse},
    rng::{InjuryRng, SeverityRng, ShotRng},
    sample_cone::concentration_p,
    stability::{StabilityTerms, terrain_brace::terrain_braces},
    tuning::CombatTuning,
    weapon::FireModeSpec,
};

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
pub(in crate::shot_pipeline::fire) struct TargetGeometry {
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
    pub(in crate::shot_pipeline::fire) fn compose(
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
pub(in crate::shot_pipeline::fire) struct RoundSetup<'a> {
    pub(in crate::shot_pipeline::fire) snapshot: &'a ShooterSnapshot,
    pub(in crate::shot_pipeline::fire) geometry: TargetGeometry,
    pub(in crate::shot_pipeline::fire) mode:     &'a FireModeSpec,
}

/// Resolve **one round** of the burst — compose its [`ShotInputs`], run E2
/// [`resolve_coarse`], and fold E3 [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) onto the struck ganger.
///
/// Composes every [`ShotInputs`] field (AC5 / AC8): the shooter's pos/facing/stance,
/// the target geometry, `cone` = [`cone_for`] at `prior_shots`, `p` =
/// [`concentration_p`], `recoil_climb` = the `tuning.cone_stability.recoil_climb`
/// leaf, and `recoil_growth` from [`stability_for`]. EVERY outcome folds through the
/// ONE [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) delegation dispatch (GTW-573): a [`ShotKind::Ganger`]
/// outcome first resolves the struck target's borrowed views off the TARGET query (a
/// struck entity that is not a queryable target folds to [`HitReport::no_effect`],
/// never a panic); a [`ShotKind::Cover`] / [`ShotKind::Slab`] / [`ShotKind::Ground`]
/// outcome spends its ledger HP / records the round's `weapon_damage` accrual
/// (GTW-364/365/366); a clean [`ShotKind::Miss`] folds to no effect inside the same
/// dispatch. Shot draws come from the injected [`ShotRng`](crate::rng::ShotRng);
/// severity draws from the injected [`SeverityRng`](crate::rng::SeverityRng).
///
/// Returns the frozen primary [`HitReport`], the `AoE` **splash** reports (GTW-541 —
/// EMPTY for a [`HitType::Single`](crate::weapon::HitType::Single) round, so the
/// single-target path is byte-identical), **and** the round's
/// [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — the already-computed E2
/// trajectory geometry [`fire`](super::super::fire) collects so
/// [`dispatch_fire`](crate::acts::dispatch_fire) can emit a per-round
/// [`ShotFired`](crate::shot_fired::ShotFired) (GTW-290). The outcome is returned
/// verbatim, NOT recomputed — the fold below already consumes it.
///
/// GTW-541 (`AoE` CORE of GTW-41): after the primary impact fold, if the fired mode's
/// [`HitType`](crate::weapon::HitType) is not
/// [`Single`](crate::weapon::HitType::Single), the template's affected cells are
/// enumerated ([`aoe_affected`](crate::aoe::aoe_affected)) and EACH occupant (skipping
/// the already-folded direct target and non-ganger cells) is routed through the EXISTING
/// [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) damage path EXACTLY ONCE, faction-blind (friendly fire hits all
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
pub(in crate::shot_pipeline::fire) fn resolve_round(
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
    // GTW-573 C7: build the FOUR zero-identity stability terms ONCE per round — the weapon's
    // stable tag, the GTW-392 terrain brace, the GTW-549 per-item brace attachment (`None` =
    // the zero identity), and the GTW-543 emplacement term (mounted → the tunable bonus, else
    // the zero identity) — and feed the SAME bundle to cone_for AND the recoil-recompute
    // stability_for below, so cone width and recoil damping are consistent by construction.
    let stability_terms = StabilityTerms {
        stable: snapshot.stable,
        terrain_braced,
        brace_bonus: snapshot.brace_bonus.unwrap_or_else(WeaponBraceBonus::none),
        emplacement: snapshot.emplacement_stability(tuning),
    };
    let cone = cone_for(
        &shooter_view,
        snapshot.weapon_stats(),
        setup.mode,
        prior_shots,
        grids.cover,
        stability_terms,
        tuning,
    );
    let (_cone_mult, recoil_growth) =
        stability_for(&shooter_view, stability_terms, grids.cover, tuning);
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

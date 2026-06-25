//! The E2.9 **coarse-pipeline entry point** — `resolve_coarse` and its
//! [`ShotOutcome`] result.
//!
//! This composes the whole §1-§4 coarse pipeline into ONE call
//! (`docs/combat/resolution.md` line 158):
//! `resolve_coarse` derives the muzzle (E2.4 [`muzzle_position`]) → the
//! climb-tilted central axis (E2.4 [`climb_aim_dir`] off the muzzle→aim axis from
//! [`target_aim_point`]) → the in-cone sample (E2.5 [`sample_cone_vector`], with
//! `θ_cone` from E2.3 and `p` from E2.5 as composed inputs) → the march (E2.7
//! [`march_vector`]) → and, **only** when the round stops on a ganger, the §4
//! part roll (E2.8 [`roll_body_part`]). It returns a [`ShotOutcome`] — the outcome
//! kind plus the struck model object, the `(Cell, Level)`, the [`BodyPart`] for a
//! ganger outcome, the crossed-cell [`HeightBand`], the muzzle [`SimPos`], and the
//! sampled trajectory unit-direction.
//!
//! **The change-driven contract (the GTW-6 / GTW-12 ruling; the headless,
//! change-driven sim↔app seam recorded in ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).** `resolve_coarse` TAKES the
//! already-maintained
//! [`OccupancyGrid`] / [`SurfaceGrid`] / [`CoverLedger`] as parameters and
//! **never rebuilds them per shot** — it only ever reads them (and the march
//! reads the occupant silhouette band off [`OccupancyGrid::occupant_band`],
//! published by the change-driven sync). There is no per-shot rebuild path in the
//! signature.
//!
//! **The carries-resolved-VALUES exception (the model/view id-not-value boundary;
//! ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).**
//! [`ShotOutcome`] is the deliberate exception to the id-not-value boundary: it
//! carries the struck model object itself (the ganger [`Entity`] / the
//! [`CoverEntry`] / the surface cell) so ids ride along for the presenter's
//! logging and FX, while every position stays in **sim units** (a cubic-voxel
//! [`SimPos`] / a unit-[`bevy::math::Vec3`] direction) — never a screen
//! coordinate, no pixel of any kind.
//!
//! **The E3 / E4 boundary.** This is the coarse pipeline ONLY: it stops at the
//! resolved outcome + part. It applies **no** damage / severity (E3) and runs
//! **no** TU / ammo bookkeeping (E4) — it mutates nothing but the injected
//! [`SimRng`]'s draw cursor. Every random draw (the cone sample and the part
//! roll) bottoms out in that one injected RNG, so the same [`crate::rng::BattleSeed`]
//! reproduces the same [`ShotOutcome`] (`docs/testing.md`'s seeded-replay
//! property).

use bevy::prelude::Entity;

use crate::{
    armor::BodyPart,
    central_axis::{climb_aim_dir, muzzle_position, target_aim_point},
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverLedger, HeightBand},
    ganger::{Facing, Position, Stance},
    hit_location::roll_body_part,
    march::{MarchKind, MarchResult, march_vector},
    metric::{Cell, CellLevel, Level, SimPos},
    occupancy::OccupancyGrid,
    rng::SimRng,
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    stability::RecoilGrowth,
    surface::SurfaceGrid,
    tuning::{CombatTuning, RecoilClimb},
};

/// What the resolved shot **struck** — the coarse outcome kind plus the struck
/// model object itself (`docs/combat/resolution.md` line 158).
///
/// A named domain enum (no-bare-types: the shot verdict is a domain value, not a
/// bare tag) carrying the struck object where there is one — the ganger's
/// [`Entity`] handle (NEVER a numeric id — GTW-10 / GTW-12), the [`CoverEntry`]
/// read from the [`CoverLedger`], or the surface `(cell, level)` for a slab /
/// ground strike. This is the deliberate carries-resolved-VALUES exception to the
/// model/view id-not-value boundary (ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`): the struck id rides along for the
/// presenter's logging / FX.
///
/// Mirrors the march's [`MarchKind`] but resolves the bare `Slab` / `Ground` /
/// `Miss` march verdicts into the surface cell each names (or none for a clean
/// miss), so the outcome carries the struck object the ticket specifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    /// The round impacted a **ganger** — carries its [`Entity`] handle (the struck
    /// actor; never a numeric id). The [`ShotOutcome::body_part`] is `Some` ONLY for
    /// this variant.
    Ganger(Entity),
    /// The round impacted a piece of **cover** — carries the [`CoverEntry`] read from
    /// the [`CoverLedger`] for the struck `(cell, level)`.
    Cover(CoverEntry),
    /// The round was stopped by an intact floor / roof **slab** — carries the surface
    /// `(cell, level)` the slab spans.
    Slab(CellLevel),
    /// The round left the **bottom** of the grid and struck the **ground** — carries
    /// the surface `(cell, level)` it exited through (damaged, never destroyed).
    Ground(CellLevel),
    /// A clean **miss** — the round cleared everything and left the grid off the top
    /// or laterally. No struck object; the trajectory is still carried for the
    /// presenter's FX.
    Miss,
}

/// The resolved outcome of one coarse shot — the entire E2 pipeline's verdict
/// (`docs/combat/resolution.md` line 158).
///
/// Every field is a named domain value (no-bare-types), and every position is in
/// **sim units** — a cubic-voxel [`SimPos`] / a unit-[`Vec3`](bevy::math::Vec3)
/// direction, **never a screen coordinate**: positions in the outcome are
/// battle-space units, never screen coords (the render-free authoritative model's
/// units; ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`; zero pixels). The
/// [`body_part`](ShotOutcome::body_part) is `Some` **only** for a
/// [`ShotKind::Ganger`] outcome (the §4 part roll runs only when the march stops
/// on a ganger).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotOutcome {
    /// What the round struck (with the struck ganger / cover / surface-cell
    /// payload).
    pub kind:       ShotKind,
    /// The `(cell, level)` ground cell of the outcome.
    pub cell:       Cell,
    /// The storey [`Level`] of the outcome (paired with [`cell`](ShotOutcome::cell)
    /// to give the `(Cell, Level)` the ticket specifies).
    pub level:      Level,
    /// The struck [`BodyPart`] — `Some` ONLY for a [`ShotKind::Ganger`] outcome
    /// (the §4 weighted roll), `None` for every other kind.
    pub body_part:  Option<BodyPart>,
    /// The round's clearance band at the crossed cell (the march's crossed-cell
    /// [`HeightBand`]).
    pub band:       HeightBand,
    /// The 3D muzzle point the shot was fired from, in sim units (a cubic-voxel
    /// [`SimPos`]).
    pub muzzle:     SimPos,
    /// The sampled trajectory — the ONE 3D unit-direction the cone draw produced
    /// (the [`ShotDir`] the march flew). A sim-space unit vector, never a pixel.
    pub trajectory: ShotDir,
}

/// The **per-shot description** [`resolve_coarse`] resolves — the shooter state,
/// the target the shot is aimed at, and the already-composed flight parameters for
/// this one round.
///
/// This is the GTW-179 bundle: the inputs that DESCRIBE the shot itself, grouped
/// apart from the world state ([`OccupancyGrid`] / [`SurfaceGrid`] / [`CoverLedger`]),
/// the config ([`CombatTuning`]), and the entropy ([`SimRng`]) — those stay their
/// own [`resolve_coarse`] parameters because they are NOT part of the shot
/// description (the §"change-driven contract" boundary; the change-driven sim↔app
/// seam recorded in ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`). Every
/// field is a named domain value (no-bare-types): no bare primitive
/// or `glam` leaf, each reusing the existing E1/E2 newtype.
///
/// The flight params arrive **already composed** (the ticket's "composed inputs"):
/// [`cone`](ShotInputs::cone) is the E2.3 [`ConeAngle`] (`θ_cone`, the cone WIDTH
/// already folded down from its five §1a factors — including the weapon's per-mode
/// [`ModeConeMult`](crate::weapon::ModeConeMult) term), and [`p`](ShotInputs::p) is
/// the E2.5 [`ConcentrationP`].
/// [`resolve_coarse`] composes nothing further from them — it consumes the
/// finished width + concentration the upstream §1 layer produced.
///
/// A plain `pub` struct of named-type fields (the bundle is a transparent argument
/// record, not itself a wrapped domain scalar); the fields are `Copy`, so it is
/// taken by `&ShotInputs`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotInputs {
    /// The shooter's grid [`Position`] — the muzzle origin cell (E2.4).
    pub shooter_position: Position,
    /// The shooter's [`Facing`] — picks the per-facing forward muzzle offset (E2.4).
    pub shooter_facing:   Facing,
    /// The shooter's [`Stance`] — picks the per-stance muzzle level-fraction (E2.4).
    pub shooter_stance:   Stance,
    /// The target's grid [`Position`] — the aim cell (E2.4).
    pub target_position:  Position,
    /// The target's [`Stance`] — picks the silhouette-top aim level-fraction (E2.4).
    pub target_stance:    Stance,
    /// The [`HeightBand`] of any cover occupying the **target** cell (so a
    /// deliberately shot crate aims at its own band midpoint), or `None` for a bare
    /// ganger target.
    pub cover_band:       Option<HeightBand>,
    /// The composed cone WIDTH `θ_cone` ([`ConeAngle`], from E2.3) — already folded
    /// from its §1a factors (the weapon's
    /// [`ModeConeMult`](crate::weapon::ModeConeMult) fire-mode term among them); the
    /// in-cone sample is drawn within it (E2.5).
    pub cone:             ConeAngle,
    /// The composed concentration exponent `p` ([`ConcentrationP`], from E2.5) —
    /// how tightly the sample biases toward the cone's dead center.
    pub p:                ConcentrationP,
    /// The number of rounds already fired this action ([`PriorShots`]) — drives the
    /// recoil-climb tilt (zero on the first round → the untilted axis).
    pub prior_shots:      PriorShots,
    /// The weapon's per-shot recoil [`RecoilClimb`] coefficient — the climb-tilt term.
    pub recoil_climb:     RecoilClimb,
    /// The stability-curve [`RecoilGrowth`] damper — damps the climb tilt for a
    /// steadier shooter.
    pub recoil_growth:    RecoilGrowth,
}

/// Decompose a march `at` [`CellLevel`] into the [`Cell`] + [`Level`] the
/// [`ShotOutcome`] carries.
///
/// The march keeps `at.z` in `0..MAX_LEVELS`, so the `u8` conversion always
/// succeeds; a `try_from` failure (a can't-happen out-of-range storey) degrades to
/// level `0` rather than panicking, keeping the resolver panic-free.
fn split_cell_level(at: CellLevel) -> (Cell, Level) {
    let storey = u8::try_from(at.z).unwrap_or(0);
    (Cell::new(at.x, at.y), Level::new(storey))
}

/// Resolve **one coarse shot** end to end and return its [`ShotOutcome`]
/// (`docs/combat/resolution.md` line 158).
///
/// The pipeline, in order:
///
/// 1. **Muzzle** — [`muzzle_position`] off the shooter's `(position, facing,
///    stance)` (E2.4): the shooter cell-center + per-facing forward offset, z = the
///    per-stance muzzle level-fraction.
/// 2. **Climb-tilted central axis** — [`target_aim_point`] off the target's
///    `(position, stance, cover_band)` gives the aim point, and [`climb_aim_dir`]
///    builds the unit muzzle→aim axis tilted UP by `prior_shots × recoil_climb ×
///    recoil_growth` (E2.4); zero prior shots → the untilted axis exactly.
/// 3. **In-cone sample** — [`sample_cone_vector`] draws ONE 3D unit direction about
///    that axis (E2.5), with the composed `θ_cone` ([`ConeAngle`], from E2.3) and
///    `p` ([`ConcentrationP`], from E2.5) — the cone width and concentration arrive
///    already composed (the ticket's composed inputs). The draw comes from the
///    injected [`SimRng`].
/// 4. **March** — [`march_vector`] flies that trajectory through the passed
///    [`OccupancyGrid`] / [`SurfaceGrid`] / [`CoverLedger`] (E2.7), reporting the
///    first thing the round fails to clear; the shooter's own cell never blocks its
///    own shot. The grids are **read, never rebuilt** (the change-driven contract).
/// 5. **Part roll** — ONLY when the march stops on a ganger, [`roll_body_part`]
///    picks the struck [`BodyPart`] from `tuning.body_part_weights` (E2.8), again
///    via the injected [`SimRng`]. A non-ganger outcome carries `None`.
///
/// The per-shot description — the shooter / target geometry and the composed
/// flight params — arrives bundled in [`ShotInputs`] (GTW-179): `shot.cone`
/// (`θ_cone`) and `shot.p` are the composed inputs the ticket specifies (the E2.3
/// cone width and E2.5 concentration, fed in); `shot.prior_shots` /
/// `shot.recoil_climb` / `shot.recoil_growth` drive the recoil-climb tilt;
/// `shot.cover_band` is the [`HeightBand`] of any cover occupying the **target**
/// cell (so a deliberately shot crate aims at its own band midpoint), or `None` for
/// a bare ganger target. The grids, the [`CombatTuning`], and the [`SimRng`] stay
/// their own parameters — they are world state + config + entropy, NOT part of the
/// shot description.
///
/// `is_dead` is the caller's read-only, RNG-free corpse predicate, threaded
/// straight into the [`march_vector`] step (GTW-317): a round that would strike a
/// ganger for which `is_dead(entity)` is `true` passes **through** the corpse and
/// continues to the next blocker, while a live occupant — including a
/// [`LifeState::Downed`](crate::ganger::LifeState::Downed) one — still stops the
/// round (only [`LifeState::Dead`](crate::ganger::LifeState::Dead) is skipped). The
/// predicate is consulted ONLY inside the march; it takes no draw, so seeded replay
/// is unaffected and the part-roll / no-draw discipline below is unchanged.
///
/// **Mutates nothing** but the injected [`SimRng`]'s draw cursor: no damage /
/// severity (E3) and no TU / ammo bookkeeping (E4) — the target's `Hp` / `Wounds`
/// / `WornArmor` and the shooter's `Tu` are untouched. Every position in the
/// result is a sim-unit [`SimPos`] / unit-[`bevy::math::Vec3`]; **zero pixels**. Same
/// [`crate::rng::BattleSeed`] → same [`ShotOutcome`] for identical inputs (the seeded-replay
/// property).
#[must_use]
pub fn resolve_coarse(
    shot: &ShotInputs,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    rng: &mut SimRng,
    is_dead: impl Fn(Entity) -> bool,
) -> ShotOutcome {
    // 1. The 3D muzzle point (E2.4).
    let muzzle = muzzle_position(
        shot.shooter_position,
        shot.shooter_facing,
        shot.shooter_stance,
        tuning,
    );

    // 2. The climb-tilted central axis: the aim point (E2.4), then the recoil-climb
    //    tilt off the muzzle→aim axis (zero prior shots → the untilted axis exactly).
    let aim_point = target_aim_point(
        shot.target_position,
        shot.target_stance,
        shot.cover_band,
        tuning,
    );
    let aim_dir = climb_aim_dir(
        muzzle,
        aim_point,
        shot.prior_shots,
        shot.recoil_climb,
        shot.recoil_growth,
    );

    // 3. The in-cone sample — ONE 3D unit trajectory about that axis (E2.5), drawn
    //    from the injected SimRng with the composed θ_cone + p.
    let trajectory = sample_cone_vector(aim_dir, shot.cone, shot.p, rng.rng());

    // 4. March the trajectory through the passed grids (E2.7) — read, never rebuilt.
    //    The shooter's own cell never blocks its own shot.
    let shooter_cell = *shot.shooter_position;
    let march = march_vector(
        muzzle,
        trajectory.vec(),
        occupancy,
        surface,
        cover,
        tuning,
        shooter_cell,
        // GTW-317 dead-occupant skip: thread the caller's real `life == Dead` predicate
        // straight into the march. A corpse is transparent; a live (incl. Downed)
        // occupant still stops the round. The predicate takes no draw, so seeded replay
        // is unchanged.
        is_dead,
    );

    outcome_from_march(march, muzzle, trajectory, tuning, rng)
}

/// Assemble the [`ShotOutcome`] from the [`MarchResult`] — resolving the march
/// verdict into a [`ShotKind`] (carrying the struck object) and running the §4
/// part roll ONLY for a ganger.
///
/// Split out so the climb / sample / march composition above stays readable and
/// under the clippy line cap. The part roll draws from the injected `rng`
/// (`tuning.body_part_weights`) exactly when the round stopped on a ganger; every
/// other kind carries `None` (AC #5).
fn outcome_from_march(
    march: MarchResult,
    muzzle: SimPos,
    trajectory: ShotDir,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> ShotOutcome {
    let (cell, level) = split_cell_level(march.at);

    // 5. The §4 part roll runs ONLY when the march stops on a ganger; every other
    //    kind carries no body part (AC #5). The struck surface cell rides along on a
    //    slab / ground outcome (the carries-resolved-VALUES exception, AC #1).
    let (kind, body_part) = match march.kind {
        MarchKind::Ganger(entity) => {
            let part = roll_body_part(&tuning.body_part_weights, rng.rng());
            (ShotKind::Ganger(entity), Some(part))
        }
        MarchKind::Cover(entry) => (ShotKind::Cover(entry), None),
        MarchKind::Slab => (ShotKind::Slab(march.at), None),
        MarchKind::Ground => (ShotKind::Ground(march.at), None),
        MarchKind::Miss => (ShotKind::Miss, None),
    };

    ShotOutcome {
        kind,
        cell,
        level,
        body_part,
        band: march.band,
        muzzle,
        trajectory,
    }
}

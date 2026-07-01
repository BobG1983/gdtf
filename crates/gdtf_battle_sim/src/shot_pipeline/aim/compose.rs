//! The two composers — [`stability_for`] (the stability read off ganger state +
//! model cover) and [`cone_for`] (the full §1a dispersion-cone WIDTH chain), each
//! wrapping the landed E2 verbs and re-deriving none of the math.

use crate::{
    aim::shooter::Shooter,
    cone::{ConeAngle, PriorShots, aim_cone_mult, cone_angle},
    cover::CoverLedger,
    faced_cell::faced_cell,
    metric::CellLevel,
    stability::{
        ConeMult, EmplacementStability, RecoilGrowth, SuppressionStability, TerrainBraced,
        stability,
    },
    tuning::CombatTuning,
    weapon::{FireModeSpec, Stable, WeaponStats},
};

/// Compose a shooter's §1a stability read off its ganger state and the model
/// cover ledger — the shared method the HUD stability readout and E4.5 `fire()`
/// both call (`docs/combat/resolution.md` §"What's pure math vs sim").
///
/// Finds the faced cell via E4.2 [`faced_cell`] (one step along the shooter's
/// [`crate::ganger::Facing`] from its [`crate::ganger::Position`], on its own
/// storey), peeks the model [`CoverLedger`] there for the faced
/// [`crate::cover::CoverEntry`] (`None` when no cover is faced — the ledger is read
/// via [`CoverLedger::peek`](crate::cover::CoverLedger::peek), never rebuilt), and
/// calls the landed [`stability`] verbatim with that cover, the shooter's
/// [`crate::ganger::Stance`], the weapon's `stable` tag, the GTW-392
/// [`TerrainBraced`] decision, the emplacement seam ([`EmplacementStability::none`] —
/// no emplacement entities exist yet), and the GTW-526 suppression seam. Returns
/// the `(cone_mult, recoil_growth)` pair the §1a cone chain reads.
///
/// GTW-526: the suppression term is resolved HERE from the shooter's
/// [`Suppressed`](crate::ganger::Suppressed) state — a suppressed shooter passes the
/// NEGATED tunable [`SuppressionStabilityPenalty`](crate::tuning::SuppressionStabilityPenalty)
/// (a subtractive [`SuppressionStability`] term that LOWERS the score → a wider cone),
/// an un-suppressed shooter passes [`SuppressionStability::none`] (the zero identity, so
/// the score is byte-identical to before the seam). Both the HUD preview and E4.5
/// `fire()` reach `stability` through THIS composer, so both reflect the penalty.
///
/// This **wraps** [`stability`], re-deriving none of the §1a math: the brace gate
/// engages when the faced cover's [`crate::cover::HeightBand`] satisfies the
/// per-stance gate OR the weapon is `stable` OR the shooter is `terrain_braced`,
/// already inside `stability`. Angular / dimensionless — **zero pixels**.
///
/// `stable` is the weapon's [`Stable`] tag — a stable weapon braces
/// unconditionally. `terrain_braced` is the GTW-392 [`TerrainBraced`] decision
/// from the fire call site; the HUD passes the live value it computed from the
/// grids. This composer receives no [`crate::weapon::Weapon`] (only the
/// shooter's ganger state), so both tags are explicit params; [`cone_for`] sources
/// `stable` off the weapon it holds. Every coefficient and both curves come from
/// `tuning`'s `cone_stability` sub-field; nothing tunable is hardcoded.
#[must_use]
pub fn stability_for(
    shooter: &Shooter,
    stable: Stable,
    terrain_braced: TerrainBraced,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> (ConeMult, RecoilGrowth) {
    // The faced cell (E4.2) — one horizontal step along the shooter's facing, on
    // its own storey; the cell whose cover the §1a brace gate reads.
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    // Peek the model ledger for the faced cover (read-only, never rebuilt — the
    // change-driven contract). `None` ⇒ no cover faced ⇒ the brace is withheld
    // (unless the weapon is `stable` or the shooter is terrain-braced, both of which
    // brace unconditionally inside `stability`).
    let faced = cover.peek(&CellLevel::new(cell, level));
    // GTW-526: resolve the suppression term from the shooter's Suppressed state. A
    // suppressed shooter loses the tunable penalty (NEGATED into a subtractive
    // contribution — a pinned shooter shoots wider); an un-suppressed shooter passes the
    // zero identity, so its score is byte-identical to a run without this seam.
    let suppression = match shooter.suppressed {
        Some(_) => SuppressionStability::new(-*tuning.reaction.suppression_penalty),
        None => SuppressionStability::none(),
    };
    // Wrap the landed verb verbatim — no emplacements exist yet, so pass the
    // identity emplacement seam.
    stability(
        stable,
        terrain_braced,
        *shooter.stance,
        faced,
        EmplacementStability::none(),
        suppression,
        &tuning.cone_stability,
    )
}

/// Compose a shooter's §1a dispersion-cone WIDTH `θ_cone` off its ganger state,
/// weapon / fire-mode data, the burst's prior-shot count, and the model cover —
/// the shared method the HUD readout and E4.5 `fire()` both call
/// (`docs/combat/resolution.md` §1a; §"What's pure math vs sim").
///
/// Composes the full §1a chain by **wrapping** the landed verbs, re-deriving none
/// of the math:
///
/// 1. [`stability_for`] for the `(cone_mult, recoil_growth)` pair (which itself
///    finds the faced cell, reads the model cover, and — GTW-526 — folds in the
///    suppression penalty when the shooter is
///    [`Suppressed`](crate::ganger::Suppressed), widening the cone).
/// 2. [`aim_cone_mult`] over the shooter's [`crate::ganger::Aiming`] flag for the
///    aim term (aimed ×0.6 / hip-fired ×1, read from `tuning`).
/// 3. [`cone_angle`] over the five §1a factors: the weapon's [`BaseSpread`], the
///    fire mode's [`ModeConeMult`], the burst's [`PriorShots`], the weapon's
///    [`Kickback`], the `recoil_growth` + `cone_mult` from step 1, and the aim
///    term from step 2.
///
/// Returns the named [`ConeAngle`] (radians — angular, **zero pixels**); every
/// factor magnitude comes from weapon / fire-mode / tuning data, none hardcoded.
///
/// The weapon's §1a stability contribution is its [`Stable`] tag, sourced off the
/// `weapon` [`WeaponStats`] borrow-view here (GTW-200: the weapon is ECS components,
/// read through the view) and threaded to [`stability_for`] (a stable weapon braces
/// unconditionally) — there is no weapon-points value any more. `terrain_braced` is
/// the GTW-392 [`TerrainBraced`] decision from the fire call site, threaded through
/// so the cone reads the stair brace. `mode` is the selected fire mode's
/// [`FireModeSpec`] (its [`crate::weapon::ModeConeMult`] is the firemode term);
/// `prior_shots` is the count of rounds already fired this action (zero on the first
/// round → the recoil term is the identity ×1).
///
/// [`BaseSpread`]: crate::weapon::BaseSpread
/// [`Kickback`]: crate::weapon::Kickback
/// [`ModeConeMult`]: crate::weapon::ModeConeMult
#[must_use]
pub fn cone_for(
    shooter: &Shooter,
    weapon: WeaponStats<'_>,
    mode: &FireModeSpec,
    prior_shots: PriorShots,
    cover: &CoverLedger,
    terrain_braced: TerrainBraced,
    tuning: &CombatTuning,
) -> ConeAngle {
    // Step 1 — the stability read (faced cell + model cover + terrain brace, inside
    // stability_for); the weapon's `stable` tag and terrain_braced are the two brace
    // sources fed through the single OR-combined brace_engages gate.
    let (cone_mult, recoil_growth) =
        stability_for(shooter, *weapon.stable, terrain_braced, cover, tuning);
    // Step 2 — the aim term off the shooter's Aiming flag (read from tuning).
    let aim = aim_cone_mult(*shooter.aiming, &tuning.cone_stability);
    // Step 3 — fold the five §1a factors into θ_cone via the landed cone_angle.
    cone_angle(
        *weapon.base_spread,
        mode.cone_mult,
        prior_shots,
        *weapon.kickback,
        recoil_growth,
        cone_mult,
        aim,
    )
}

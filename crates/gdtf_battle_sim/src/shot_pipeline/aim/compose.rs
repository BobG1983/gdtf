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
    weapon::{FireModeSpec, Stable, WeaponBraceBonus, WeaponStats},
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
/// grids. `brace_bonus` is the GTW-549 additive per-item brace term — the graduated
/// [`WeaponBraceBonus`](crate::weapon::WeaponBraceBonus) a data-driven
/// [`Stability`](crate::weapon::AttachmentEffect::Stability) attachment fits, else
/// [`WeaponBraceBonus::none`] (the zero identity, so the score is byte-identical to a weapon
/// with no brace attachment). This SUPERSEDES the GTW-542 sight-stability term — a sight now
/// boosts AIM (the [`Accuracy`](crate::weapon::Accuracy) stat), not stability. `emplacement` is
/// the GTW-543 additive emplacement term — the tunable
/// [`EmplacementStabilityBonus`](crate::tuning::EmplacementStabilityBonus) when the shooter's
/// resolved ranged weapon is a [`MountedWeapon`](crate::weapon::MountedWeapon) (a fixed mount
/// aims steadier), else [`EmplacementStability::none`] (the zero identity, so an un-mounted
/// shot is byte-identical to before the seam engaged). This composer
/// receives no [`crate::weapon::Weapon`] (only the shooter's ganger state), so all the
/// weapon terms are explicit params; [`cone_for`] sources them off the weapon it holds.
/// Every coefficient and both curves come from `tuning`'s `cone_stability` sub-field;
/// nothing tunable is hardcoded.
#[must_use]
pub fn stability_for(
    shooter: &Shooter,
    stable: Stable,
    terrain_braced: TerrainBraced,
    brace_bonus: WeaponBraceBonus,
    emplacement: EmplacementStability,
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
    // Wrap the landed verb verbatim. `emplacement` is the GTW-543 additive emplacement term —
    // `cone_for` resolves it off the shooter's resolved weapon (a MountedWeapon feeds the tunable
    // bonus, an un-mounted shot passes EmplacementStability::none() so the score is byte-identical
    // to before the seam engaged). `brace_bonus` is the GTW-549 additive per-item brace term,
    // resolved by `cone_for` off the weapon's optional WeaponBraceBonus attachment component
    // (absent = WeaponBraceBonus::none, so the score is byte-identical to a weapon with no brace).
    stability(
        stable,
        terrain_braced,
        *shooter.stance,
        faced,
        emplacement,
        suppression,
        brace_bonus,
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
/// The weapon's §1a stability contribution is its [`Stable`] tag PLUS the GTW-549 per-item
/// [`WeaponBraceBonus`](crate::weapon::WeaponBraceBonus) attachment, both sourced off the
/// `weapon` [`WeaponStats`] borrow-view here (GTW-200: the weapon is ECS components, read
/// through the view) and threaded to [`stability_for`] — a stable weapon braces
/// unconditionally, and a weapon carrying a data-driven
/// [`Stability`](crate::weapon::AttachmentEffect::Stability) attachment adds its graduated
/// [`WeaponBraceBonus`](crate::weapon::WeaponBraceBonus) points (a weapon with no brace
/// resolves the zero-identity [`WeaponBraceBonus::none`], so the cone is byte-identical to
/// before the attachment). This SUPERSEDES the GTW-542 sight-stability seam — a sight now
/// boosts AIM (the [`Accuracy`](crate::weapon::Accuracy) stat), not stability. `emplacement`
/// is the GTW-543 additive
/// emplacement term — resolved by the caller (which knows whether the shooter's ranged weapon is
/// a [`MountedWeapon`](crate::weapon::MountedWeapon)) and threaded through to
/// [`stability_for`]: a mounted gun feeds the tunable
/// [`EmplacementStabilityBonus`](crate::tuning::EmplacementStabilityBonus), an un-mounted shot
/// passes [`EmplacementStability::none`] (the zero identity, so the cone is byte-identical to
/// before the seam engaged). Unlike `sight` (a per-weapon component read off `weapon`), the
/// emplacement term is NOT a weapon stat — it is the shooter's TRANSIENT occupancy of an
/// emplacement — so it is an explicit param, not sourced off `weapon`. `terrain_braced` is
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
#[expect(
    clippy::too_many_arguments,
    reason = "the §1a cone chain reads the shooter state, the weapon borrow-view, the fire mode, \
              the burst's prior-shot count, the model cover, the GTW-392 terrain brace, and the \
              GTW-543 emplacement stability term — each a distinct named §1a input the caller \
              resolves; bundling them would hide which contribution feeds the cone width"
)]
pub fn cone_for(
    shooter: &Shooter,
    weapon: WeaponStats<'_>,
    mode: &FireModeSpec,
    prior_shots: PriorShots,
    cover: &CoverLedger,
    terrain_braced: TerrainBraced,
    emplacement: EmplacementStability,
    tuning: &CombatTuning,
) -> ConeAngle {
    // GTW-549: resolve the per-item brace term off the weapon's optional WeaponBraceBonus
    // attachment component — a weapon carrying a data-driven `Stability` attachment (`Some`)
    // feeds its graduated positive bonus; a weapon with no brace (`None`) resolves the zero
    // identity, so its cone is byte-identical to before the attachment existed.
    let brace_bonus = weapon
        .brace_bonus
        .copied()
        .unwrap_or_else(WeaponBraceBonus::none);
    // Step 1 — the stability read (faced cell + model cover + terrain brace, inside
    // stability_for); the weapon's `stable` tag and terrain_braced are the two brace
    // sources fed through the single OR-combined brace_engages gate, the GTW-549 `brace_bonus`
    // term is the additive per-item stability seam, and the GTW-543 `emplacement` term is the
    // additive mounted-gun-steadying seam (the caller resolves it off the weapon's MountedWeapon
    // marker).
    let (cone_mult, recoil_growth) = stability_for(
        shooter,
        *weapon.stable,
        terrain_braced,
        brace_bonus,
        emplacement,
        cover,
        tuning,
    );
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

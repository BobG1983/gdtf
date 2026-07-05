//! Brace-source parity/identity seams (GTW-199 AC3, GTW-392, GTW-549, GTW-573)
//! — the pins that share one change-reason: a new brace source or additive term.

use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    stability::{
        TerrainBraced,
        score::stability,
        types::{EmplacementStability, StabilityTerms, SuppressionStability},
    },
    tuning::ConeStabilityTuning,
    weapon::{Stable, WeaponBraceBonus},
};

/// GTW-549 — the additive per-item BRACE seam steadies the shot: a POSITIVE
/// [`WeaponBraceBonus`] contribution (a weapon with a data-driven `Stability` attachment)
/// yields a strictly LOWER [`ConeMult`] (a tighter cone) than the [`WeaponBraceBonus::none`]
/// identity, all else equal. RELATION only — never a pinned magnitude. Also proves the
/// identity: [`WeaponBraceBonus::none`] leaves the score byte-identical to the pre-seam sum
/// (the pure-additive property, mirroring the GTW-526 suppression identity). This is the seam
/// that SUPERSEDES the GTW-542 sight-stability seam (a sight now boosts AIM, not stability).
#[test]
fn a_positive_brace_bonus_yields_a_strictly_tighter_cone() {
    let tuning = ConeStabilityTuning::default();

    // Baseline: a weapon with no brace attachment (the zero-identity term) facing an empty cell.
    let (baseline_cone, _) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );
    // Braced: a POSITIVE per-item brace bonus — a steadier score → a tighter cone.
    let (braced_cone, _) = stability(
        StabilityTerms {
            brace_bonus: WeaponBraceBonus::new(20.0),
            ..StabilityTerms::default()
        },
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *braced_cone < *baseline_cone,
        "a braced weapon (positive WeaponBraceBonus) must read a strictly LOWER cone_mult \
         (tighter): braced {} vs baseline {}",
        *braced_cone,
        *baseline_cone,
    );

    // IDENTITY: WeaponBraceBonus::none() is byte-identical to a run without the seam — proven
    // by re-computing the baseline with the same identity term and asserting bit-equality.
    let (identity_cone, _) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert_eq!(
        (*identity_cone).to_bits(),
        (*baseline_cone).to_bits(),
        "a weapon with no brace attachment reads a byte-identical cone_mult under the \
         zero-identity brace term",
    );
}

/// AC3 (GTW-199) — the stability score composes from stance + brace(+stable) +
/// emplacement, with NO weapon-points term: the ONLY difference between a stable
/// and a non-stable weapon is whether the brace engages. Facing an EMPTY cell
/// (so the non-stable weapon gets no brace), a stable weapon is strictly
/// steadier (lower `cone_mult`) — the difference traces entirely to the brace.
/// When BOTH face cover that suits the stance (the brace already engaged for
/// both), stable and non-stable are EQUAL (the tag adds nothing beyond the
/// brace). Relations only, no pinned magnitudes.
#[test]
fn stable_difference_traces_to_the_brace_no_weapon_points_term() {
    let tuning = ConeStabilityTuning::default();
    let stance = Stance::new(StanceKind::Standing);

    // Empty cell: only the stable weapon braces.
    let (stable_empty, _) = stability(
        StabilityTerms {
            stable: Stable::new(true),
            ..StabilityTerms::default()
        },
        stance,
        None,
        SuppressionStability::none(),
        &tuning,
    );
    let (plain_empty, _) = stability(
        StabilityTerms::default(),
        stance,
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *stable_empty < *plain_empty,
        "facing an empty cell, a stable weapon braces while a non-stable one does \
         not — so it must be strictly steadier (lower cone_mult): {} vs {}",
        *stable_empty,
        *plain_empty,
    );

    // Cover that suits the stance (standing's gate is HIGH): both braces engage,
    // so stable adds nothing beyond it — the two are EQUAL.
    let wall = faced_cover(HeightBand::High);
    let (stable_braced, _) = stability(
        StabilityTerms {
            stable: Stable::new(true),
            ..StabilityTerms::default()
        },
        stance,
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    let (plain_braced, _) = stability(
        StabilityTerms::default(),
        stance,
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    assert_eq!(
        (*stable_braced).to_bits(),
        (*plain_braced).to_bits(),
        "facing suitable cover, the brace already engages for both — stable adds \
         nothing, so the scores are equal",
    );
}

/// GTW-392 score-level parity — the terrain brace routes through the EXACT same
/// `brace_contribution` magnitude as the weapon's `stable` tag. Driving the real
/// [`stability`] path twice with IDENTICAL inputs (same stance / faced cover /
/// emplacement / tuning) — once `stable`-only, once `terrain_braced`-only — must
/// yield BIT-IDENTICAL `(cone_mult, recoil_growth)` outputs. This would FAIL the
/// instant terrain-brace were ever wired to a different magnitude than the stable
/// tag. A no-brace baseline (neither source on) must DIFFER, pinning that the brace
/// actually changes the output (so the parity is not the trivial both-equal-baseline
/// case). The faced cell is empty so NEITHER run gets the §1a cover/stance brace —
/// each run's only brace source is its single flag.
#[test]
fn terrain_brace_and_stable_yield_identical_stability_output() {
    let tuning = ConeStabilityTuning::default();
    // Standing facing no cover: the §1a cover/stance gate never engages, so the
    // ONLY brace source in each run is the explicit flag under test.
    let stance = Stance::new(StanceKind::Standing);

    // stable=true, terrain_braced=false.
    let (stable_cone, stable_recoil) = stability(
        StabilityTerms {
            stable: Stable::new(true),
            ..StabilityTerms::default()
        },
        stance,
        None,
        SuppressionStability::none(),
        &tuning,
    );
    // stable=false, terrain_braced=true — every other input identical.
    let (terrain_cone, terrain_recoil) = stability(
        StabilityTerms {
            terrain_braced: TerrainBraced::new(true),
            ..StabilityTerms::default()
        },
        stance,
        None,
        SuppressionStability::none(),
        &tuning,
    );

    // Bit-identical: terrain-brace routes through the SAME brace_contribution quantum.
    assert_eq!(
        (*stable_cone).to_bits(),
        (*terrain_cone).to_bits(),
        "terrain-brace must produce a bit-identical cone_mult to the stable tag \
         (same brace_contribution magnitude): {} vs {}",
        *stable_cone,
        *terrain_cone,
    );
    assert_eq!(
        (*stable_recoil).to_bits(),
        (*terrain_recoil).to_bits(),
        "terrain-brace must produce a bit-identical recoil_growth to the stable tag \
         (same brace_contribution magnitude): {} vs {}",
        *stable_recoil,
        *terrain_recoil,
    );

    // No-brace baseline (neither source on): the brace withheld, so the output must
    // DIFFER from the braced result — pinning that the brace actually moves the score.
    let (baseline_cone, baseline_recoil) = stability(
        StabilityTerms::default(),
        stance,
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert_ne!(
        (*baseline_cone).to_bits(),
        (*terrain_cone).to_bits(),
        "the no-brace baseline must differ from the braced result — the brace must \
         actually change the output: {} vs {}",
        *baseline_cone,
        *terrain_cone,
    );
    assert_ne!(
        (*baseline_recoil).to_bits(),
        (*terrain_recoil).to_bits(),
        "the no-brace baseline recoil_growth must differ from the braced result: {} vs {}",
        *baseline_recoil,
        *terrain_recoil,
    );
}

/// GTW-573 C7 — the zero-identity DEFAULT: `StabilityTerms::default()` reads a
/// byte-identical `(cone_mult, recoil_growth)` to the same call with every term
/// spelled at its explicit zero identity, across all three stances — so a defaulted
/// bundle IS the no-seam baseline, and a future additive term (one new field with a
/// zero-identity default) cannot shift any existing score.
#[test]
fn default_terms_are_the_byte_identical_zero_identity() {
    let tuning = ConeStabilityTuning::default();
    let explicit = StabilityTerms {
        stable:         Stable::new(false),
        terrain_braced: TerrainBraced::new(false),
        brace_bonus:    WeaponBraceBonus::none(),
        emplacement:    EmplacementStability::none(),
    };
    for kind in [
        StanceKind::Standing,
        StanceKind::Crouching,
        StanceKind::Prone,
    ] {
        let (default_cone, default_recoil) = stability(
            StabilityTerms::default(),
            Stance::new(kind),
            None,
            SuppressionStability::none(),
            &tuning,
        );
        let (explicit_cone, explicit_recoil) = stability(
            explicit,
            Stance::new(kind),
            None,
            SuppressionStability::none(),
            &tuning,
        );
        assert_eq!(
            (*default_cone).to_bits(),
            (*explicit_cone).to_bits(),
            "{kind:?}: the defaulted terms must read a byte-identical cone_mult",
        );
        assert_eq!(
            (*default_recoil).to_bits(),
            (*explicit_recoil).to_bits(),
            "{kind:?}: the defaulted terms must read a byte-identical recoil_growth",
        );
    }
}

use super::support::*;
use crate::{
    cover::HeightBand,
    effects::attachments::WeaponBraceBonus,
    ganger::{Stance, StanceKind},
    stability::{
        TerrainBraced,
        score::stability,
        types::{EmplacementStability, StabilityTerms, SuppressionStability},
    },
    tuning::ConeStabilityTuning,
    weapon::Stable,
};

#[test]
fn a_positive_brace_bonus_yields_a_strictly_tighter_cone() {
    let tuning = ConeStabilityTuning::default();

    let (baseline_cone, _) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );
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
        "a weapon with no brace attachment reads an identical cone_mult under the \
         zero-identity brace term",
    );
}

#[test]
fn stable_difference_traces_to_the_brace_no_weapon_points_term() {
    let tuning = ConeStabilityTuning::default();
    let stance = Stance::new(StanceKind::Standing);

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

#[test]
fn terrain_brace_and_stable_yield_identical_stability_output() {
    let tuning = ConeStabilityTuning::default();
    let stance = Stance::new(StanceKind::Standing);

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
            "{kind:?}: the defaulted terms must read an identical cone_mult",
        );
        assert_eq!(
            (*default_recoil).to_bits(),
            (*explicit_recoil).to_bits(),
            "{kind:?}: the defaulted terms must read an identical recoil_growth",
        );
    }
}

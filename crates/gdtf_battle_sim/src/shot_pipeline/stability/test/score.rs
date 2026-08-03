use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    stability::{
        score::stability,
        types::{StabilityTerms, SuppressionStability},
    },
    tuning::ConeStabilityTuning,
};

#[test]
fn stability_produces_both_named_outputs() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    let (steady_cone, steady_recoil) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    let (shaky_cone, shaky_recoil) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );

    assert!((*steady_cone).is_finite() && (*steady_recoil).is_finite());
    assert!((*shaky_cone).is_finite() && (*shaky_recoil).is_finite());
    assert!(
        *steady_cone < *shaky_cone,
        "a steadier situation must yield a narrower cone_mult",
    );
    assert!(
        *steady_recoil < *shaky_recoil,
        "a steadier situation must yield less recoil_growth",
    );
}

#[test]
fn steadier_stance_yields_narrower_cone() {
    let tuning = ConeStabilityTuning::default();
    let cone = |kind| {
        stability(
            StabilityTerms::default(),
            Stance::new(kind),
            None,
            SuppressionStability::none(),
            &tuning,
        )
        .0
    };
    let prone = *cone(StanceKind::Prone);
    let kneel = *cone(StanceKind::Crouching);
    let stand = *cone(StanceKind::Standing);
    assert!(
        prone < kneel && kneel < stand,
        "prone < kneel < stand in cone_mult (steadier → narrower): {prone} {kneel} {stand}",
    );
}

#[test]
fn steadier_score_yields_strictly_less_recoil_growth() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    let (_, braced_prone) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    let (_, standing_unbraced) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *braced_prone < *standing_unbraced,
        "a braced/prone shooter must climb strictly less than a standing/un-braced one",
    );
}

//! The `stability()` score composition + posture monotonicity (C1/C3/C4).

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

/// C1 — `stability(...)` returns BOTH named outputs computed from the three
/// contributions, normalised over 100, read off the two tuning curves. A
/// relation, not a magnitude: a fully steady situation (prone + braced on a
/// HIGH wall) must produce a finite `cone_mult` and `recoil_growth`, and (with
/// the default curves' steadier-is-smaller shape) a smaller `cone_mult` than a
/// fully shaky one. Value-agnostic on the actual numbers.
#[test]
fn stability_produces_both_named_outputs() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    // Steadiest: prone, braced on a HIGH wall (satisfies the prone gate, LOW+).
    let (steady_cone, steady_recoil) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    // Shakiest: standing, no cover faced (no brace), no emplacement help.
    let (shaky_cone, shaky_recoil) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );

    // Both outputs are produced and finite.
    assert!((*steady_cone).is_finite() && (*steady_recoil).is_finite());
    assert!((*shaky_cone).is_finite() && (*shaky_recoil).is_finite());
    // Steadier yields a narrower cone and less climb (relation, not magnitude).
    assert!(
        *steady_cone < *shaky_cone,
        "a steadier situation must yield a narrower cone_mult",
    );
    assert!(
        *steady_recoil < *shaky_recoil,
        "a steadier situation must yield less recoil_growth",
    );
}

/// C3 — steadier postures yield a steadier score: prone < kneel < stand in
/// `cone_mult` for the same weapon and NO brace (none faced). A
/// monotonic-relation test over the three stances — ordering only, never
/// magnitudes.
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

/// C4 — `recoil_growth` is the score's SECOND curve output, and a steadier
/// score yields strictly-LESS climb: a braced/prone shooter's `recoil_growth`
/// is strictly below a standing/un-braced one's (ordering, not magnitude).
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

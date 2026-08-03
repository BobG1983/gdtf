use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    stability::{
        TerrainBraced,
        gate::brace_engages,
        score::stability,
        types::{StabilityTerms, SuppressionStability},
    },
    tuning::ConeStabilityTuning,
    weapon::Stable,
};

#[test]
fn brace_applied_exactly_when_faced_band_satisfies_gate() {
    let tuning = ConeStabilityTuning::default();

    let cases = [
        (StanceKind::Standing, HeightBand::High, HeightBand::Mid),
        (StanceKind::Crouching, HeightBand::Mid, HeightBand::Low),
    ];
    for (kind, satisfying, failing) in cases {
        let sat = faced_cover(satisfying);
        let fail = faced_cover(failing);
        let (braced, _) = stability(
            StabilityTerms::default(),
            Stance::new(kind),
            Some(&sat),
            SuppressionStability::none(),
            &tuning,
        );
        let (unbraced, _) = stability(
            StabilityTerms::default(),
            Stance::new(kind),
            Some(&fail),
            SuppressionStability::none(),
            &tuning,
        );
        assert!(
            *braced < *unbraced,
            "{kind:?}: a satisfying faced band must brace (steadier, lower cone_mult) vs a failing one",
        );
    }

    let low = faced_cover(HeightBand::Low);
    let (prone_braced, _) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Prone),
        Some(&low),
        SuppressionStability::none(),
        &tuning,
    );
    let (prone_unbraced, _) = stability(
        StabilityTerms::default(),
        Stance::new(StanceKind::Prone),
        None,
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *prone_braced < *prone_unbraced,
        "prone braced on a LOW wall must be steadier than prone facing no cover",
    );
}

#[test]
fn stable_weapon_braces_unconditionally_non_stable_does_not() {
    let tuning = ConeStabilityTuning::default();
    let unsuitable = faced_cover(HeightBand::Mid);
    let stance = StanceKind::Standing;

    assert!(
        *brace_engages(
            Stable::new(true),
            TerrainBraced::new(false),
            stance,
            None,
            &tuning
        ),
        "a stable weapon must brace even facing an EMPTY cell",
    );
    assert!(
        !*brace_engages(
            Stable::new(false),
            TerrainBraced::new(false),
            stance,
            None,
            &tuning
        ),
        "a non-stable weapon must NOT brace facing an empty cell",
    );

    assert!(
        *brace_engages(
            Stable::new(true),
            TerrainBraced::new(false),
            stance,
            Some(&unsuitable),
            &tuning
        ),
        "a stable weapon must brace even facing cover that does not suit the stance",
    );
    assert!(
        !*brace_engages(
            Stable::new(false),
            TerrainBraced::new(false),
            stance,
            Some(&unsuitable),
            &tuning
        ),
        "a non-stable weapon must NOT brace facing cover that does not suit the stance",
    );
}

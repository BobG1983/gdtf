//! The `brace_engages` per-stance band gate + the stable-tag bypass (C2,
//! GTW-199 AC2).

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

/// C2 — the auto-brace contribution is applied EXACTLY when the faced cell's
/// `CoverEntry.height_band` satisfies the per-stance gate, and withheld
/// otherwise. For each stance, pair a SATISFYING faced band with a
/// NON-satisfying one (both read directly off the `CoverEntry`) and assert
/// brace-on is steadier (lower `cone_mult`) than brace-off. No band literal — the
/// satisfying/failing bands are derived from the tuning gate itself.
#[test]
fn brace_applied_exactly_when_faced_band_satisfies_gate() {
    let tuning = ConeStabilityTuning::default();

    // (stance, a band that SATISFIES its gate, a band that FAILS it).
    // Derived from the doc gate prone↔LOW+, kneel↔MID+, stand↔HIGH: the gate
    // band itself satisfies; the band one rank below it fails (stand's gate is
    // HIGH, so MID fails; kneel's is MID, so LOW fails; prone's is LOW — nothing
    // is below LOW, so prone's "fail" is the no-cover-faced case, asserted
    // separately below).
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

    // Prone's gate is LOW+, so a LOW wall satisfies it; no cover faced does not.
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

/// AC2 (GTW-199) — `brace_engages` is `true` for a STABLE weapon facing a cell
/// that does NOT suit the stance (an empty cell, and cover one rank below the
/// gate), and `false` for a NON-stable weapon in the same situation. A relation
/// over the gate, never a pinned score. The stable tag bypasses the §1a cover /
/// stance gate entirely.
#[test]
fn stable_weapon_braces_unconditionally_non_stable_does_not() {
    let tuning = ConeStabilityTuning::default();
    // Standing's gate is HIGH (resolution.md §1a), so a MID wall does NOT suit
    // it — and an empty cell never suits any stance.
    let unsuitable = faced_cover(HeightBand::Mid);
    let stance = StanceKind::Standing;

    // Empty cell (no faced cover): stable braces, non-stable does not.
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

    // Cover present but its band does NOT suit the stance: same relation.
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

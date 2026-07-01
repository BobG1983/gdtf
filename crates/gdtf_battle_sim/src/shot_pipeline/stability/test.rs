use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, HeightBand},
    ganger::{Stance, StanceKind},
    stability::{
        TerrainBraced,
        curve::read_curve,
        gate::brace_engages,
        score::stability,
        types::{
            ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, SuppressionStability,
        },
    },
    tuning::{ConeStabilityTuning, StabilityCurve},
    weapon::Stable,
};

/// An arbitrary faced-cover entry at `band` — NOT shipped magnitudes; the
/// stability layer only reads `height_band`, so the HP/armor are filler.
fn faced_cover(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        band,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    )
}

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
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    // Shakiest: standing, no cover faced (no brace), no emplacement help.
    let (shaky_cone, shaky_recoil) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Standing),
        None,
        EmplacementStability::none(),
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
            Stable::new(false),
            TerrainBraced::new(false),
            Stance::new(kind),
            Some(&sat),
            EmplacementStability::none(),
            SuppressionStability::none(),
            &tuning,
        );
        let (unbraced, _) = stability(
            Stable::new(false),
            TerrainBraced::new(false),
            Stance::new(kind),
            Some(&fail),
            EmplacementStability::none(),
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
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Prone),
        Some(&low),
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    let (prone_unbraced, _) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Prone),
        None,
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *prone_braced < *prone_unbraced,
        "prone braced on a LOW wall must be steadier than prone facing no cover",
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
            Stable::new(false),
            TerrainBraced::new(false),
            Stance::new(kind),
            None,
            EmplacementStability::none(),
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
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    let (_, standing_unbraced) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Standing),
        None,
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    assert!(
        *braced_prone < *standing_unbraced,
        "a braced/prone shooter must climb strictly less than a standing/un-braced one",
    );
}

/// C5 — the score is clamped/normalised to 0–100 BEFORE the curve read: a
/// degenerate over-100 contribution sum does not read off the curve's end or
/// panic. Two wildly over-100 sums whose RAW totals differ (different stance /
/// brace) must still produce the SAME outputs, proving both clamp to the score
/// ceiling (and never a panic / NaN). The over-100 sum is driven by the
/// emplacement seam — there is no weapon-points term any more.
#[test]
fn over_100_sum_clamps_and_does_not_run_off_the_curve() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    // A wildly over-100 raw sum: prone + braced + a huge emplacement term.
    let (over_cone, over_recoil) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Prone),
        Some(&wall),
        EmplacementStability::new(10_000.0),
        SuppressionStability::none(),
        &tuning,
    );
    // A DIFFERENT over-100 raw sum (standing, no brace) — also driven over the
    // ceiling by a huge emplacement term, so it too clamps to 100.
    let (ceil_cone, ceil_recoil) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        Stance::new(StanceKind::Standing),
        None,
        EmplacementStability::new(10_000.0),
        SuppressionStability::none(),
        &tuning,
    );

    assert!((*over_cone).is_finite() && (*over_recoil).is_finite());
    // Both clamp to the score ceiling, so the curve reads are identical — the
    // over-100 sums did not run off the curve's end.
    assert_eq!((*over_cone).to_bits(), (*ceil_cone).to_bits());
    assert_eq!((*over_recoil).to_bits(), (*ceil_recoil).to_bits());

    // And the clamped score really is the ceiling (not beyond it).
    assert_eq!(
        (*StabilityScore::clamped(10_000.0)).to_bits(),
        StabilityScore::MAX.to_bits(),
    );
}

/// The piecewise-linear curve read is clamped at both endpoints and
/// interpolates between authored points — the curve *form*. Built from an
/// ARBITRARY two-point curve parsed from a bare-scalar RON fragment (the tuning
/// leaf coords are `#[serde(transparent)]` with private inners, so RON is the
/// in-test build path — and these are arbitrary literals, never shipped values):
/// below the first score returns the first output, above the last returns the
/// last, and a midpoint score returns the midpoint output. An empty curve
/// returns the identity 1.0.
#[test]
fn read_curve_clamps_endpoints_and_interpolates() {
    // Arbitrary two-point curve: score 20→output 2.0, score 60→output 4.0.
    let parsed = ron::from_str::<StabilityCurve>(
        r"[ ( score: 20.0, output: 2.0 ), ( score: 60.0, output: 4.0 ) ]",
    );
    assert!(parsed.is_ok(), "arbitrary curve must parse: {parsed:?}");
    let Ok(curve) = parsed else {
        return;
    };

    // Below the first point's score → first output (left clamp).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(0.0))).to_bits(),
        2.0_f32.to_bits(),
    );
    // Above the last point's score → last output (right clamp).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(100.0))).to_bits(),
        4.0_f32.to_bits(),
    );
    // Midpoint score (40 is halfway between 20 and 60) → midpoint output (3.0).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(40.0))).to_bits(),
        3.0_f32.to_bits(),
    );

    // Empty (degenerate) curve → identity 1.0, never a panic.
    let empty = StabilityCurve::new(Vec::new());
    assert_eq!(
        (*read_curve(&empty, StabilityScore::clamped(50.0))).to_bits(),
        1.0_f32.to_bits(),
    );
}

/// The output newtypes' derived [`Deref`] reaches their inner value, and the
/// two outputs are DISTINCT types (no-bare-types rule 3). Built from arbitrary
/// literals — pins the Deref mechanism + target type, not a magnitude.
#[test]
fn output_newtypes_deref_to_inner() {
    assert_eq!((*ConeMult::new(0.7)).to_bits(), 0.7_f32.to_bits());
    assert_eq!((*RecoilGrowth::new(0.3)).to_bits(), 0.3_f32.to_bits());
    assert_eq!(
        (*EmplacementStability::new(8.0)).to_bits(),
        8.0_f32.to_bits()
    );
    assert_eq!((*EmplacementStability::none()).to_bits(), 0.0_f32.to_bits());
    // GTW-526: the suppression term derefs to its (negative-in-practice) inner, and its
    // identity is 0.0 (the byte-identity term for an un-suppressed shooter).
    assert_eq!(
        (*SuppressionStability::new(-40.0)).to_bits(),
        (-40.0_f32).to_bits()
    );
    assert_eq!((*SuppressionStability::none()).to_bits(), 0.0_f32.to_bits());
    assert_eq!(
        (*StabilityScore::clamped(50.0)).to_bits(),
        50.0_f32.to_bits()
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
        brace_engages(
            Stable::new(true),
            TerrainBraced::new(false),
            stance,
            None,
            &tuning
        ),
        "a stable weapon must brace even facing an EMPTY cell",
    );
    assert!(
        !brace_engages(
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
        brace_engages(
            Stable::new(true),
            TerrainBraced::new(false),
            stance,
            Some(&unsuitable),
            &tuning
        ),
        "a stable weapon must brace even facing cover that does not suit the stance",
    );
    assert!(
        !brace_engages(
            Stable::new(false),
            TerrainBraced::new(false),
            stance,
            Some(&unsuitable),
            &tuning
        ),
        "a non-stable weapon must NOT brace facing cover that does not suit the stance",
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
        Stable::new(true),
        TerrainBraced::new(false),
        stance,
        None,
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    let (plain_empty, _) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        stance,
        None,
        EmplacementStability::none(),
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
        Stable::new(true),
        TerrainBraced::new(false),
        stance,
        Some(&wall),
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    let (plain_braced, _) = stability(
        Stable::new(false),
        TerrainBraced::new(false),
        stance,
        Some(&wall),
        EmplacementStability::none(),
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
        Stable::new(true),
        TerrainBraced::new(false),
        stance,
        None,
        EmplacementStability::none(),
        SuppressionStability::none(),
        &tuning,
    );
    // stable=false, terrain_braced=true — every other input identical.
    let (terrain_cone, terrain_recoil) = stability(
        Stable::new(false),
        TerrainBraced::new(true),
        stance,
        None,
        EmplacementStability::none(),
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
        Stable::new(false),
        TerrainBraced::new(false),
        stance,
        None,
        EmplacementStability::none(),
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

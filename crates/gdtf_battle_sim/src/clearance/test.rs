//! Relocated unit tests for the clearance-banding rule (GTW-201 wave 22 — moved
//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    clearance::{
        Clearance, band::band_rank, round_band_for_cell, round_band_fraction, round_clears_occupant,
    },
    cover::{BandFraction, HeightBand, band_for},
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, pos_to_cell},
    tuning::CombatTuning,
};

/// A `SimPos` at the center of cell `(x, y)` on storey `level`, raised to
/// `above_floor` sim units **within** that level — the round's continuous z is
/// `level + above_floor`, so its within-level fraction is `above_floor`.
fn round_at(x: i32, y: i32, level: u8, above_floor: f32) -> SimPos {
    SimPos::new(
        x as f32 + 0.5,
        y as f32 + 0.5,
        f32::from(level) + above_floor,
    )
}

/// AC #1 — a fraction below / at / above the tuning edges classifies LOW / MID /
/// HIGH, on EACH of several storeys (the per-level floor offset applied
/// correctly). The probe fractions are DERIVED from the tuning edges, never
/// literals (AC #4 discipline), so a tuning edit moves them with the boundaries.
///
/// The probes are placed in each band's INTERIOR (derived from the edges) so the
/// classification survives the `z − k` fraction reconstruction at every storey —
/// f32 cannot represent `level + edge` exactly, so a knife-edge probe sitting
/// *exactly* on an edge can undershoot it by an ULP after the floor subtraction
/// at higher storeys. The exact-edge `<` semantic itself is pinned separately at
/// storey 0 (where `z − 0` reconstructs exactly) by
/// [`exact_edge_classifies_to_the_upper_band_at_storey_zero`].
#[test]
fn round_band_classifies_below_between_above_edges_on_every_storey() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
    // Interior probes derived from the edges: half the LOW→MID edge is well
    // inside LOW; the midpoint of the two edges is well inside MID; the midpoint
    // of the MID→HIGH edge and the storey ceiling (1.0) is well inside HIGH.
    let in_low = *edges.low_mid * 0.5;
    let in_mid = f32::midpoint(*edges.low_mid, *edges.mid_high);
    let in_high = f32::midpoint(*edges.mid_high, 1.0);

    // Several storeys across the valid 0..MAX_LEVELS range — the per-level floor
    // offset must not change the band a given above-floor fraction lands in.
    for level in [0u8, 1, 3, MAX_LEVELS - 1] {
        assert_eq!(
            round_band_for_cell(round_at(2, 2, level, in_low), &tuning),
            HeightBand::Low,
            "below the LOW→MID edge must be LOW on storey {level}",
        );
        assert_eq!(
            round_band_for_cell(round_at(2, 2, level, in_mid), &tuning),
            HeightBand::Mid,
            "between the edges must be MID on storey {level}",
        );
        assert_eq!(
            round_band_for_cell(round_at(2, 2, level, in_high), &tuning),
            HeightBand::High,
            "above the MID→HIGH edge must be HIGH on storey {level}",
        );
    }
}

/// AC #1 — the exact-edge `<` semantic (a fraction *at* an edge belongs to the
/// UPPER band, matching the landed `band_for`'s `<` comparison) pinned at storey
/// 0, where the within-level fraction `z − 0` reconstructs the probe exactly.
/// This is the boundary case the per-storey test deliberately keeps off the
/// knife-edge; here it is exact.
#[test]
fn exact_edge_classifies_to_the_upper_band_at_storey_zero() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
    // On storey 0, `round_at(.., 0, f)` has z == f exactly → fraction == f.
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, *edges.low_mid), &tuning),
        HeightBand::Mid,
        "a fraction exactly at the LOW→MID edge is MID (band_for uses `<`)",
    );
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, *edges.mid_high), &tuning),
        HeightBand::High,
        "a fraction exactly at the MID→HIGH edge is HIGH (band_for uses `<`)",
    );
}

/// AC #1 / #5 — the within-level fraction is `pos.z − k` with `k` from
/// [`pos_to_cell`]: the SAME above-floor height yields the SAME band regardless
/// of storey (the floor offset is applied per level, not accumulated). Uses a
/// band-interior probe so the across-storey equality is not at the precision
/// knife-edge.
#[test]
fn same_above_floor_height_yields_same_band_across_storeys() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
    // A probe well inside the MID band (midpoint of the two edges).
    let above_floor = f32::midpoint(*edges.low_mid, *edges.mid_high);
    let band_l0 = round_band_for_cell(round_at(5, 5, 0, above_floor), &tuning);
    for level in 1u8..MAX_LEVELS {
        assert_eq!(
            round_band_for_cell(round_at(5, 5, level, above_floor), &tuning),
            band_l0,
            "the per-level floor offset must not change the band (storey {level})",
        );
    }
    assert_eq!(band_l0, HeightBand::Mid);
}

/// AC #4 — band edges come solely from `tuning.projectile_band_edges`: editing
/// an edge moves the boundary. A fraction that is MID under the default edges
/// becomes LOW once the LOW→MID edge is raised above it — proving no fraction is
/// hardcoded and `band_for` reads the live tuning.
#[test]
fn editing_a_tuning_edge_moves_the_band_boundary() {
    use crate::tuning::BandEdge;

    let default = CombatTuning::default();
    // A probe sitting exactly on the default LOW→MID edge → MID by default.
    let probe = *default.projectile_band_edges.low_mid;
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, probe), &default),
        HeightBand::Mid,
        "the probe is MID under the default edges",
    );

    // Raise the LOW→MID edge above the probe → the SAME round is now LOW.
    let mut raised = CombatTuning::default();
    raised.projectile_band_edges.low_mid = BandEdge::new(probe + 0.1);
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, probe), &raised),
        HeightBand::Low,
        "raising the LOW→MID edge above the probe must reclassify it LOW",
    );
}

/// AC #5 — a degenerate / below-floor fraction (`z < k` for the crossed level)
/// classifies LOW gracefully, no panic. A round whose continuous z sits BELOW
/// the crossed cell's floor yields a negative within-level fraction, which
/// `band_for` puts in LOW (strictly below the LOW→MID edge).
#[test]
fn below_floor_fraction_classifies_low_without_panic() {
    let tuning = CombatTuning::default();
    // The round's z is below storey 3's floor (z = 2.5 < 3.0). pos_to_cell floors
    // it into storey 2, so its OWN within-level fraction is 0.5 — but the
    // band-FRACTION helper measures against the crossed level it floors into;
    // here we exercise an explicitly negative within-level fraction the way a
    // march measuring against a HIGHER crossed level would see it.
    let negative = BandFraction::new(-0.5);
    assert_eq!(
        band_for(negative, &tuning),
        HeightBand::Low,
        "a negative within-level fraction must classify LOW",
    );

    // And the cell-level helper itself never panics for an at-floor round (the
    // exact storey floor → fraction 0.0 → LOW).
    let at_floor = SimPos::new(2.5, 2.5, 3.0);
    assert_eq!(round_band_for_cell(at_floor, &tuning), HeightBand::Low);
    // round_band_fraction at the floor is exactly 0.0 (no accumulated offset).
    assert_eq!(
        (*round_band_fraction(at_floor)).to_bits(),
        0.0_f32.to_bits()
    );
}

/// AC #2 — the full LOW/MID/HIGH × LOW/MID/HIGH matrix: EXACTLY the strictly
/// higher (round band ranks above occupant band) cells clear; every
/// equal-or-lower cell impacts.
#[test]
fn clearance_matrix_clears_exactly_the_strictly_higher_cells() {
    let bands = [HeightBand::Low, HeightBand::Mid, HeightBand::High];
    for round in bands {
        for occupant in bands {
            let expected = if band_rank(round) > band_rank(occupant) {
                Clearance::Clears
            } else {
                Clearance::Impacts
            };
            assert_eq!(
                round_clears_occupant(round, occupant),
                expected,
                "round {round:?} vs occupant {occupant:?}: strictly-higher clears, else impacts",
            );
        }
    }

    // Spell out the three diagonal (equal) cells impact, and the three strictly
    // higher cells clear, so the matrix's intent is explicit and not just a
    // restated formula.
    assert_eq!(
        round_clears_occupant(HeightBand::High, HeightBand::Mid),
        Clearance::Clears,
    );
    assert_eq!(
        round_clears_occupant(HeightBand::High, HeightBand::Low),
        Clearance::Clears,
    );
    assert_eq!(
        round_clears_occupant(HeightBand::Mid, HeightBand::Low),
        Clearance::Clears,
    );
    assert_eq!(
        round_clears_occupant(HeightBand::Mid, HeightBand::High),
        Clearance::Impacts,
    );
    assert_eq!(
        round_clears_occupant(HeightBand::Low, HeightBand::High),
        Clearance::Impacts,
    );
}

/// AC #3 — the prone-can't-clear-LOW consequence, reproduced by band-vs-band with
/// no exemption list: a round in the LOW band does NOT clear a LOW occupant
/// (equal ⇒ impacts). This grounds §2's retirement of special-casing — a *prone*
/// shooter fires LOW and so genuinely cannot clear even LOW cover.
#[test]
fn low_round_does_not_clear_low_occupant() {
    assert_eq!(
        round_clears_occupant(HeightBand::Low, HeightBand::Low),
        Clearance::Impacts,
        "a LOW round vs a LOW occupant is EQUAL, so it impacts (prone can't clear LOW)",
    );
}

/// AC #1 end-to-end — classifying a crossed cell then running the predicate: a
/// HIGH round sails over a LOW occupant at a crossed cell. Threads
/// [`round_band_for_cell`] (via a derived above-floor fraction) into
/// [`round_clears_occupant`], proving the two pieces compose as the march will
/// use them.
#[test]
fn classified_round_clears_a_lower_occupant_end_to_end() {
    let tuning = CombatTuning::default();
    // An above-floor fraction at-or-above the MID→HIGH edge → HIGH round.
    let above_floor = *tuning.projectile_band_edges.mid_high;
    let round = round_at(8, 8, 2, above_floor);
    let round_band = round_band_for_cell(round, &tuning);
    assert_eq!(round_band, HeightBand::High);

    // A LOW occupant in the crossed cell — the HIGH round sails over.
    let occupant_band = HeightBand::Low;
    assert_eq!(
        round_clears_occupant(round_band, occupant_band),
        Clearance::Clears,
        "a HIGH round must sail over a LOW occupant at the crossed cell",
    );

    // The crossed (cell, level) the round floors into, for completeness.
    let (cell, level) = pos_to_cell(round);
    assert_eq!(
        CellLevel::new(cell, level),
        CellLevel::new(Cell::new(8, 8), Level::new(2))
    );
}

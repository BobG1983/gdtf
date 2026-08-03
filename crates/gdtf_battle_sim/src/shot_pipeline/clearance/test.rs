//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    clearance::{
        Clearance, band::band_rank, lower_band, round_band_for_cell, round_band_fraction,
        round_clears_occupant, silhouette_band,
    },
    cover::{BandFraction, HeightBand, band_for},
    ganger::StanceKind,
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, pos_to_cell},
    tuning::CombatTuning,
};

#[test]
fn silhouette_band_maps_stance_to_its_band() {
    assert_eq!(silhouette_band(StanceKind::Standing), HeightBand::High);
    assert_eq!(silhouette_band(StanceKind::Crouching), HeightBand::Mid);
    assert_eq!(silhouette_band(StanceKind::Prone), HeightBand::Low);

    assert!(
        band_rank(silhouette_band(StanceKind::Standing))
            > band_rank(silhouette_band(StanceKind::Crouching)),
        "standing presents a strictly higher silhouette than kneeling",
    );
    assert!(
        band_rank(silhouette_band(StanceKind::Crouching))
            > band_rank(silhouette_band(StanceKind::Prone)),
        "kneeling presents a strictly higher silhouette than prone",
    );
}

fn round_at(x: i32, y: i32, level: u8, above_floor: f32) -> SimPos {
    SimPos::new(
        x as f32 + 0.5,
        y as f32 + 0.5,
        f32::from(level) + above_floor,
    )
}

#[test]
fn round_band_classifies_below_between_above_edges_on_every_storey() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
    let in_low = *edges.low_mid * 0.5;
    let in_mid = f32::midpoint(*edges.low_mid, *edges.mid_high);
    let in_high = f32::midpoint(*edges.mid_high, 1.0);

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

#[test]
fn exact_edge_classifies_to_the_upper_band_at_storey_zero() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
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

#[test]
fn same_above_floor_height_yields_same_band_across_storeys() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;
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

#[test]
fn editing_a_tuning_edge_moves_the_band_boundary() {
    use crate::tuning::BandEdge;

    let default = CombatTuning::default();
    let probe = *default.projectile_band_edges.low_mid;
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, probe), &default),
        HeightBand::Mid,
        "the probe is MID under the default edges",
    );

    let mut raised = CombatTuning::default();
    raised.projectile_band_edges.low_mid = BandEdge::new(probe + 0.1);
    assert_eq!(
        round_band_for_cell(round_at(1, 1, 0, probe), &raised),
        HeightBand::Low,
        "raising the LOW→MID edge above the probe must reclassify it LOW",
    );
}

#[test]
fn below_floor_fraction_classifies_low_without_panic() {
    let tuning = CombatTuning::default();
    let negative = BandFraction::new(-0.5);
    assert_eq!(
        band_for(negative, &tuning),
        HeightBand::Low,
        "a negative within-level fraction must classify LOW",
    );

    let at_floor = SimPos::new(2.5, 2.5, 3.0);
    assert_eq!(round_band_for_cell(at_floor, &tuning), HeightBand::Low);
    assert_eq!(
        (*round_band_fraction(at_floor)).to_bits(),
        0.0_f32.to_bits()
    );
}

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

#[test]
fn low_round_does_not_clear_low_occupant() {
    assert_eq!(
        round_clears_occupant(HeightBand::Low, HeightBand::Low),
        Clearance::Impacts,
        "a LOW round vs a LOW occupant is EQUAL, so it impacts (prone can't clear LOW)",
    );
}

#[test]
fn lower_band_returns_the_minimum_of_two_bands() {
    use HeightBand::{High, Low, Mid};

    assert_eq!(lower_band(Low, Low), Low);
    assert_eq!(lower_band(Mid, Mid), Mid);
    assert_eq!(lower_band(High, High), High);

    assert_eq!(lower_band(High, Mid), Mid);
    assert_eq!(lower_band(Mid, High), Mid);
    assert_eq!(lower_band(High, Low), Low);
    assert_eq!(lower_band(Low, High), Low);
    assert_eq!(lower_band(Mid, Low), Low);
    assert_eq!(lower_band(Low, Mid), Low);

    for a in [Low, Mid, High] {
        for b in [Low, Mid, High] {
            let lo = lower_band(a, b);
            assert!(band_rank(lo) <= band_rank(a) && band_rank(lo) <= band_rank(b));
            let expected = if band_rank(a) <= band_rank(b) { a } else { b };
            assert_eq!(lo, expected, "lower_band({a:?}, {b:?})");
        }
    }
}

#[test]
fn classified_round_clears_a_lower_occupant_end_to_end() {
    let tuning = CombatTuning::default();
    let above_floor = *tuning.projectile_band_edges.mid_high;
    let round = round_at(8, 8, 2, above_floor);
    let round_band = round_band_for_cell(round, &tuning);
    assert_eq!(round_band, HeightBand::High);

    let occupant_band = HeightBand::Low;
    assert_eq!(
        round_clears_occupant(round_band, occupant_band),
        Clearance::Clears,
        "a HIGH round must sail over a LOW occupant at the crossed cell",
    );

    let (cell, level) = pos_to_cell(round);
    assert_eq!(
        CellLevel::new(cell, level),
        CellLevel::new(Cell::new(8, 8), Level::new(2))
    );
}

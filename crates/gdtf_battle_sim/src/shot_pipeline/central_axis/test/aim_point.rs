use super::support::{TOL, position};
use crate::{
    central_axis::{
        aim_point::{band_bottom_fraction, band_midpoint_fraction, band_top_fraction},
        target_aim_point,
    },
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    metric::{Cell, Level, cell_center},
    tuning::{AimHeightFrac, CombatTuning},
};

fn tuning_with_silhouette_tops(prone: f32, kneel: f32, stand: f32) -> CombatTuning {
    use crate::tuning::{SilhouetteTop, SilhouetteTops};
    let mut tuning = CombatTuning::default();
    tuning.cone_stability.silhouette_tops = SilhouetteTops {
        prone: SilhouetteTop::new(prone),
        kneel: SilhouetteTop::new(kneel),
        stand: SilhouetteTop::new(stand),
    };
    tuning
}

#[test]
fn ganger_aim_z_is_per_stance_silhouette_top_times_aim_height_frac() {
    let mut tuning = tuning_with_silhouette_tops(0.2, 0.45, 0.85);
    tuning.cone_stability.aim_height_frac = AimHeightFrac::new(0.7);
    let frac = *tuning.cone_stability.aim_height_frac;
    let level = 2u8;
    let pos = position(5, 5, level);

    let cases = [
        (StanceKind::Prone, 0.2_f32),
        (StanceKind::Crouching, 0.45_f32),
        (StanceKind::Standing, 0.85_f32),
    ];
    for (kind, top) in cases {
        let aim = target_aim_point(pos, Stance::new(kind), None, &tuning);
        let expected = top.mul_add(frac, f32::from(level));
        assert!(
            (aim.z - expected).abs() < TOL,
            "{kind:?}: aim z {} should equal level + silhouette_top × aim_height_frac {expected}",
            aim.z,
        );
    }
}

#[test]
fn ganger_aim_z_reads_each_stance_its_own_silhouette_top() {
    let tuning = tuning_with_silhouette_tops(0.1, 0.5, 0.9);
    let level = 0u8;
    let pos = position(3, 4, level);

    let z_prone = target_aim_point(pos, Stance::new(StanceKind::Prone), None, &tuning).z;
    let z_kneel = target_aim_point(pos, Stance::new(StanceKind::Crouching), None, &tuning).z;
    let z_stand = target_aim_point(pos, Stance::new(StanceKind::Standing), None, &tuning).z;
    assert!(
        z_prone < z_kneel && z_kneel < z_stand,
        "each stance must read its own SilhouetteTop (prone {z_prone} < kneel {z_kneel} < stand {z_stand})",
    );
}

#[test]
fn ganger_aim_z_doubles_when_a_stances_silhouette_top_doubles() {
    let single = tuning_with_silhouette_tops(0.3, 0.6, 0.4);
    let doubled = tuning_with_silhouette_tops(0.6, 0.6, 0.4);
    let pos = position(0, 0, 0);
    let stance = Stance::new(StanceKind::Prone);

    let z_single = target_aim_point(pos, stance, None, &single).z;
    let z_doubled = target_aim_point(pos, stance, None, &doubled).z;
    assert!(
        2.0f32.mul_add(-z_single, z_doubled).abs() < TOL,
        "doubling the prone SilhouetteTop must double the above-floor aim z: {z_doubled} vs 2×{z_single}",
    );
}

#[test]
fn ganger_aim_z_scales_with_aim_height_frac() {
    let mut low = tuning_with_silhouette_tops(0.3, 0.6, 0.95);
    low.cone_stability.aim_height_frac = AimHeightFrac::new(0.5);
    let mut high = tuning_with_silhouette_tops(0.3, 0.6, 0.95);
    high.cone_stability.aim_height_frac = AimHeightFrac::new(1.0);

    let pos = position(0, 0, 0);
    let stance = Stance::new(StanceKind::Standing);
    let z_low = target_aim_point(pos, stance, None, &low).z;
    let z_high = target_aim_point(pos, stance, None, &high).z;
    assert!(
        2.0f32.mul_add(-z_low, z_high).abs() < TOL,
        "aim z above floor must scale linearly with aim_height_frac: {z_high} vs 2×{z_low}",
    );
}

#[test]
fn ganger_aim_xy_is_cell_center() {
    let tuning = CombatTuning::default();
    let pos = position(9, 12, 1);
    let center = cell_center(Cell::new(9, 12), Level::new(1));
    let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), None, &tuning);
    assert_eq!(aim.x.to_bits(), center.x.to_bits());
    assert_eq!(aim.y.to_bits(), center.y.to_bits());
}

#[test]
fn cover_aim_z_is_within_each_bands_fraction_range() {
    let tuning = CombatTuning::default();
    let level = 3u8;
    let pos = position(6, 6, level);
    let edges = tuning.projectile_band_edges;

    for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
        let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), Some(band), &tuning);
        let above_floor = aim.z - f32::from(level);
        let bottom = *band_bottom_fraction(band, edges);
        let top = *band_top_fraction(band, edges);
        assert!(
            above_floor >= bottom && above_floor < top,
            "{band:?}: aim fraction {above_floor} must lie in [{bottom}, {top})",
        );
    }
}

#[test]
fn cover_aim_z_is_the_band_midpoint() {
    let tuning = CombatTuning::default();
    let pos = position(1, 1, 0);
    let edges = tuning.projectile_band_edges;
    for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
        let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), Some(band), &tuning);
        let expected = *band_midpoint_fraction(band, edges);
        assert!(
            (aim.z - expected).abs() < TOL,
            "{band:?}: cover aim z {} should be the band midpoint {expected}",
            aim.z,
        );
    }
}

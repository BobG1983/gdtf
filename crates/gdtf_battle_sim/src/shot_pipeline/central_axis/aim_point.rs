use crate::{
    cover::{BandFraction, HeightBand},
    ganger::{Position, Stance, StanceKind},
    metric::{SimPos, cell_center},
    tuning::{AimHeightFrac, CombatTuning, ProjectileBandEdges, SilhouetteTop},
};

const fn silhouette_top(stance: StanceKind, tuning: &CombatTuning) -> SilhouetteTop {
    let tops = &tuning.cone_stability.silhouette_tops;
    match stance {
        StanceKind::Prone => tops.prone,
        StanceKind::Crouching => tops.kneel,
        StanceKind::Standing => tops.stand,
    }
}

pub(super) fn band_top_fraction(band: HeightBand, edges: ProjectileBandEdges) -> BandFraction {
    match band {
        HeightBand::Low => BandFraction::new(*edges.low_mid),
        HeightBand::Mid => BandFraction::new(*edges.mid_high),
        HeightBand::High => BandFraction::new(1.0),
    }
}

pub(super) fn band_bottom_fraction(band: HeightBand, edges: ProjectileBandEdges) -> BandFraction {
    match band {
        HeightBand::Low => BandFraction::new(0.0),
        HeightBand::Mid => BandFraction::new(*edges.low_mid),
        HeightBand::High => BandFraction::new(*edges.mid_high),
    }
}

pub(super) fn band_midpoint_fraction(band: HeightBand, edges: ProjectileBandEdges) -> BandFraction {
    let bottom = band_bottom_fraction(band, edges);
    let top = band_top_fraction(band, edges);
    BandFraction::new(f32::midpoint(*bottom, *top))
}

#[must_use]
pub fn target_aim_point(
    position: Position,
    stance: Stance,
    cover_band: Option<HeightBand>,
    tuning: &CombatTuning,
) -> SimPos {
    let (cell, level) = position.split();

    let center = cell_center(cell, level);
    let edges = tuning.projectile_band_edges;

    let aim_fraction = if let Some(band) = cover_band {
        *band_midpoint_fraction(band, edges)
    } else {
        let top = *silhouette_top(*stance, tuning);
        top * *aim_height_frac(tuning)
    };

    SimPos::new(center.x, center.y, f32::from(*level) + aim_fraction)
}

const fn aim_height_frac(tuning: &CombatTuning) -> AimHeightFrac {
    tuning.cone_stability.aim_height_frac
}

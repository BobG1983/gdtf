//! The §1 **aim point** — [`target_aim_point`] (cell-center x/y; z is the ganger's
//! per-stance silhouette-top × aim-height-frac, or a cover band's midpoint
//! level-fraction).

use crate::{
    cover::HeightBand,
    ganger::{Position, Stance, StanceKind},
    metric::{SimPos, cell_center},
    tuning::{AimHeightFrac, CombatTuning, ProjectileBandEdges, SilhouetteTop},
};

/// The per-stance **silhouette-top** level-fraction for `stance`, read off `tuning`'s
/// dedicated [`crate::tuning::SilhouetteTops`] (the silhouette-top home GTW-164 added
/// to [`CombatTuning`]) — no magnitude lives here. The stance→field mapping matches
/// the muzzle-height mapping (prone → `prone`, kneel/`Crouching` → `kneel`, stand →
/// `stand`; battle-space.md §"Stance / cover / muzzle / aim heights").
///
/// (Replaces the former `ProjectileBandEdges`-derived silhouette band-top, which
/// conflated silhouette/aim height with the round's clearance band edges.)
const fn silhouette_top(stance: StanceKind, tuning: &CombatTuning) -> SilhouetteTop {
    let tops = &tuning.cone_stability.silhouette_tops;
    match stance {
        StanceKind::Prone => tops.prone,
        StanceKind::Crouching => tops.kneel,
        StanceKind::Standing => tops.stand,
    }
}

/// The **band-top** level-fraction of a [`HeightBand`], derived from the tunable
/// [`ProjectileBandEdges`] (resolution.md §1 / AC #3): Low-top = `low_mid`,
/// Mid-top = `mid_high`, High-top = `1.0` (the storey ceiling). Never a hardcoded
/// magnitude beyond the storey-ceiling fraction `1.0`.
pub(super) fn band_top_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    match band {
        HeightBand::Low => *edges.low_mid,
        HeightBand::Mid => *edges.mid_high,
        // The top of the HIGH band is the top of the storey — the 1.0 level-fraction
        // (a level-fraction, not a pixel): nothing within a storey sits above it.
        HeightBand::High => 1.0,
    }
}

/// The **band-bottom** level-fraction of a [`HeightBand`], derived from the tunable
/// [`ProjectileBandEdges`]: Low-bottom = `0.0` (the storey floor), Mid-bottom =
/// `low_mid`, High-bottom = `mid_high`. Paired with [`band_top_fraction`] this
/// gives each band its `[bottom, top)` level-fraction range.
pub(super) fn band_bottom_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    match band {
        // The bottom of the LOW band is the storey floor — the 0.0 level-fraction.
        HeightBand::Low => 0.0,
        HeightBand::Mid => *edges.low_mid,
        HeightBand::High => *edges.mid_high,
    }
}

/// The **midpoint** level-fraction of a [`HeightBand`] — halfway between its
/// `[bottom, top)` edges, both derived from the tunable [`ProjectileBandEdges`]
/// (AC #4: "a cover-occupied cell aims at the object's own band midpoint, derived
/// from the band edges, never a literal"). Lies strictly inside the band's range.
pub(super) fn band_midpoint_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    let bottom = band_bottom_fraction(band, edges);
    let top = band_top_fraction(band, edges);
    f32::midpoint(bottom, top)
}

/// The **aim point** a shooter centres the cone on, as a [`SimPos`] (resolution.md
/// §1 `target_aim_point`; battle-space.md §"Stance / cover / muzzle / aim heights").
///
/// The ground-plane x/y is the target cell's [`cell_center`]; the z is a
/// level-fraction derived per-case (AC #3/#4):
///
/// * **No cover faced** (`cover_band == None`) — a **ganger** target: the target's
///   `stance` selects its **per-stance silhouette-top** level-fraction from the
///   dedicated [`crate::tuning::SilhouetteTops`] tuning (prone → `prone`, kneel →
///   `kneel`, stand → `stand`), which is scaled by [`crate::tuning::AimHeightFrac`] —
///   `z = level + silhouette_top × aim_height_frac`. The silhouette top is its own
///   tuning home, NOT the round's [`ProjectileBandEdges`] clearance band edges.
/// * **Cover-occupied cell** (`cover_band == Some(band)`): aim at the cover's own
///   [`HeightBand`] **midpoint** level-fraction ([`band_midpoint_fraction`], derived
///   from the tunable [`ProjectileBandEdges`] band edges — cover height genuinely IS
///   band-based) — `z = level + band_midpoint`, so deliberately shooting a low crate
///   works at range.
///
/// `position` and `stance` are the target's per-field ECS components; `cover_band`
/// is the [`HeightBand`] of any cover occupying the target cell (e.g. read off a
/// [`crate::cover::CoverEntry`]), or `None` for a bare ganger target. Every magnitude
/// comes from `tuning`; zero pixels.
#[must_use]
pub fn target_aim_point(
    position: Position,
    stance: Stance,
    cover_band: Option<HeightBand>,
    tuning: &CombatTuning,
) -> SimPos {
    // The canonical CellLevel::split decompose through Position's deref (GTW-565).
    let (cell, level) = position.split();

    let center = cell_center(cell, level);
    let edges = tuning.projectile_band_edges;

    // A cover-occupied cell aims at the cover band's midpoint level-fraction (cover
    // height genuinely IS band-based); a bare ganger target aims at its per-stance
    // silhouette-top level-fraction (the dedicated SilhouetteTops tuning) × aim_height_frac.
    let aim_fraction = if let Some(band) = cover_band {
        band_midpoint_fraction(band, edges)
    } else {
        let top = *silhouette_top(*stance, tuning);
        top * *aim_height_frac(tuning)
    };

    SimPos::new(center.x, center.y, f32::from(*level) + aim_fraction)
}

/// The [`AimHeightFrac`] coefficient from `tuning` (the dimensionless fraction of a
/// ganger target's silhouette-top height the aim point pins). No magnitude here.
const fn aim_height_frac(tuning: &CombatTuning) -> AimHeightFrac {
    tuning.cone_stability.aim_height_frac
}

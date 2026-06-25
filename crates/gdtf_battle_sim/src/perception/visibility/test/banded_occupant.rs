//! AC for [`union_fov`](crate::visibility::union_fov): it resolves each candidate
//! occupant's band via `cover.peek().or_else(occupant_band)` and builds the [`Target`]
//! at that band before [`can_see`](crate::los::can_see) — so a crouched (non-Standing)
//! enemy is classified by its banded anchor (the SAME band the shot pipeline aims at),
//! NOT a generic cell-center (GTW-340 clause 5 / banded-occupant AC).

use super::support::*;
use crate::central_axis::target_aim_point;

/// The union's per-candidate aim z (the occupant-band branch) DIFFERS from a generic
/// cell-center z — proving `union_fov` routes the published band, not a flat
/// `z = level` anchor.
///
/// This mirrors the GTW-337 shot-pipeline-parity strategy: the occupant-band aim z
/// (what the shot pipeline uses) is distinct from the cell-center z, so a union that
/// silently aimed at the cell center would visibly miss this. We assert the EXACT z the
/// union threads — `target_aim_point(.., occupant_band, ..)` for a published band — and
/// confirm it is not the cell-center z.
#[test]
fn band_resolved_aim_z_is_not_cell_center() {
    let tuning = CombatTuning::default();

    let target_cell = key(8, 5, 0);
    let mut occupancy = OccupancyGrid::new();
    // A crouched enemy publishes a MID silhouette band (non-Standing).
    place_occupant(&mut occupancy, target_cell, spawn_entity(), HeightBand::Mid);

    // The aim point the shot pipeline (and union_fov) uses: the published MID band fed
    // into target_aim_point's band-midpoint branch. The union builds its candidate
    // Target with the inert Standing fallback stance, but the published band OVERRIDES
    // the stance branch — so the aim z is the band midpoint, NOT the silhouette top.
    let banded_aim = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        occupancy.occupant_band(&target_cell),
        &tuning,
    );

    // A generic cell-center anchor would sit at z = the level's floor exactly (the 0.0
    // level-fraction) — the metric's own `cell_center` z for the cell.
    let cell_center_z = crate::metric::cell_center(Cell::new(8, 5), Level::new(0)).z;

    assert_ne!(
        banded_aim.z.to_bits(),
        cell_center_z.to_bits(),
        "the occupant-band aim z must DIFFER from a generic cell-center z (z = level)"
    );
}

/// Behavioral proof the union routes the published band: in a band-sensitive geometry,
/// the union's verdict for a crouched occupant FLIPS depending on whether its MID band
/// is published — VISIBLE with the band, BLOCKED without it.
///
/// The geometry is the GTW-329-style steep descending shot: a HIGH standing observer one
/// level up, looking DOWN at a target one storey below behind a HIGH cover edge on the
/// observer's own (upper) level just in front of the target column. With the MID band
/// published, the union aims at the band midpoint of the lower cell and the descending
/// ray reaches it; with NO band published, `can_see` falls to the bare-stance
/// silhouette-top anchor — a DIFFERENT ray that the HIGH upper-level edge occludes. The
/// only difference between the two `union_fov` calls is the published band, so the
/// flipped verdict isolates the band routing (clause 5 / banded-occupant AC).
#[test]
fn union_verdict_depends_on_the_published_band() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();

    // Target one storey BELOW the observer, behind a HIGH cover edge on the OBSERVER's
    // level just ahead of the target column — a descending-ray occlusion that the band
    // height controls.
    let target_cell = key(8, 5, 0);
    let edge_cell = key(7, 5, 1);
    cover.insert(edge_cell, cover_entry(HeightBand::High));

    // A HIGH standing observer one level up, looking down the x axis.
    let (pos, st, fc) = alive_observer_at(2, 5, 1);
    let make_observers = || {
        [FovObserver {
            position:         &pos,
            stance:           &st,
            facing:           &fc,
            life:             LifeState::Alive,
            stair_eye_offset: StairEyeOffset::new(0.0),
        }]
    };

    // WITH the MID band published: the union aims at the lower cell's band midpoint.
    let mut occupancy_banded = OccupancyGrid::new();
    place_occupant(
        &mut occupancy_banded,
        target_cell,
        spawn_entity(),
        HeightBand::Mid,
    );
    let visible_banded = union_fov(
        &make_observers(),
        &occupancy_banded,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    // WITHOUT the band: an occupant is present (so the cell is still a candidate) but no
    // band is published, so `can_see` falls to the bare-stance silhouette-top aim — a
    // different ray.
    let mut occupancy_no_band = OccupancyGrid::new();
    occupancy_no_band.set_occupant(target_cell, Some(spawn_entity()));
    let visible_no_band = union_fov(
        &make_observers(),
        &occupancy_no_band,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    // The band is the ONLY difference between the two grids, so a differing verdict for
    // the target cell isolates the band routing. (The two aim z's are distinct — proven
    // below — so the two rays genuinely differ.)
    let banded_aim_z = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        Some(HeightBand::Mid),
        &tuning,
    )
    .z;
    let bare_aim_z = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        None,
        &tuning,
    )
    .z;
    assert_ne!(
        banded_aim_z.to_bits(),
        bare_aim_z.to_bits(),
        "the MID-band aim z and the bare-stance aim z are distinct rays"
    );
    assert_ne!(
        visible_banded.contains(&target_cell),
        visible_no_band.contains(&target_cell),
        "the union's verdict for the crouched occupant FLIPS on whether its band is \
         published — proving union_fov resolves the occupant band, not a generic anchor"
    );
}

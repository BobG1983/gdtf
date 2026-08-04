//! Arc (thrown) path sampling for grenades and similar.

use bevy::prelude::Deref;

use crate::{
    clearance::round_band_for_cell,
    march::{
        dda::MAX_STEPS,
        geom::{VoxelIndex, key_of, key_of_clamped, xy_in_grid, z_in_grid},
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, MAX_LEVELS, SimPos, SimUnit, cell_center, pos_to_cell},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct ArcFraction(f32);

impl ArcFraction {
    const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct SampleCount(u32);

impl SampleCount {
    const fn new(count: u32) -> Self {
        Self(count)
    }
}

const SAMPLE_STEP: f32 = 0.25;

const BASE_APEX: f32 = 0.5;

const APEX_PER_CELL: f32 = 0.02;

fn arc_z(z0: SimUnit, z1: SimUnit, u: ArcFraction, apex: SimUnit) -> SimUnit {
    let straight = (*z1 - *z0).mul_add(*u, *z0);
    SimUnit::new((*apex).mul_add(4.0 * *u * (1.0 - *u), straight))
}

/// Sample an arc from thrower to target and stop on the first roof slab hit.
#[must_use]
pub fn march_arc(
    thrower: CellLevel,
    target: CellLevel,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> MarchResult {
    let (thrower_cell, thrower_level) = thrower.split();
    let (target_cell, target_level) = target.split();
    let muzzle = cell_center(thrower_cell, thrower_level);
    let landing = cell_center(target_cell, target_level);

    if !*xy_in_grid(VoxelIndex::new(thrower.x), VoxelIndex::new(thrower.y))
        || !*z_in_grid(VoxelIndex::new(thrower.z))
        || !*xy_in_grid(VoxelIndex::new(target.x), VoxelIndex::new(target.y))
        || !*z_in_grid(VoxelIndex::new(target.z))
    {
        return MarchResult {
            kind:   MarchKind::Miss,
            at:     key_of_clamped(
                VoxelIndex::new(thrower.x),
                VoxelIndex::new(thrower.y),
                VoxelIndex::new(thrower.z),
            ),
            band:   round_band_for_cell(muzzle, tuning),
            impact: muzzle,
        };
    }

    let dx = landing.x - muzzle.x;
    let dy = landing.y - muzzle.y;
    let horizontal = dx.hypot(dy);
    let apex = horizontal.mul_add(APEX_PER_CELL, BASE_APEX);
    let steps = arc_sample_count(SimUnit::new(horizontal));

    let mut prev = muzzle;
    for i in 1..=*steps {
        #[expect(
            clippy::cast_precision_loss,
            reason = "steps is a small sample count bounded by MAX_STEPS; the f32 fraction is exact for this range"
        )]
        let u = ArcFraction::new((i as f32) / (*steps as f32));
        let point = SimPos::new(
            dx.mul_add(*u, muzzle.x),
            dy.mul_add(*u, muzzle.y),
            *arc_z(
                SimUnit::new(muzzle.z),
                SimUnit::new(landing.z),
                u,
                SimUnit::new(apex),
            ),
        );
        if let Some(blocked) = roof_block_between(prev, point, surface, tuning) {
            return blocked;
        }
        prev = point;
    }

    MarchResult {
        kind:   MarchKind::Ground,
        at:     CellLevel::new(target_cell, target_level),
        band:   round_band_for_cell(landing, tuning),
        impact: landing,
    }
}

fn arc_sample_count(horizontal: SimUnit) -> SampleCount {
    let raw = (*horizontal / SAMPLE_STEP).ceil();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "raw is a non-negative ceil'd count clamped to 1.0..=MAX_STEPS below, so the u32 cast cannot wrap"
    )]
    let n = raw.clamp(1.0, f32::from(u16::MAX)) as u32;
    SampleCount::new(n.clamp(1, MAX_STEPS))
}

fn roof_block_between(
    prev: SimPos,
    point: SimPos,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> Option<MarchResult> {
    let prev_z = floor_level(SimUnit::new(prev.z));
    let cur_z = floor_level(SimUnit::new(point.z));
    if prev_z == cur_z {
        return None;
    }
    let low = *prev_z.min(cur_z) + 1;
    let high = *prev_z.max(cur_z);
    if cur_z > prev_z {
        for boundary in low..=high {
            if let Some(blocked) =
                roof_block_at(prev, point, VoxelIndex::new(boundary), surface, tuning)
            {
                return Some(blocked);
            }
        }
    } else {
        for boundary in (low..=high).rev() {
            if let Some(blocked) =
                roof_block_at(prev, point, VoxelIndex::new(boundary), surface, tuning)
            {
                return Some(blocked);
            }
        }
    }
    None
}

fn roof_block_at(
    prev: SimPos,
    point: SimPos,
    boundary: VoxelIndex,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> Option<MarchResult> {
    if !*z_in_grid(boundary) {
        return None;
    }
    let plane = f32::from(u8::try_from(*boundary).ok()?);
    let t = ((plane - prev.z) / (point.z - prev.z)).clamp(0.0, 1.0);
    let cross = SimPos::new(
        (point.x - prev.x).mul_add(t, prev.x),
        (point.y - prev.y).mul_add(t, prev.y),
        plane,
    );
    let (cell, _) = pos_to_cell(cross);
    let slab_key = key_of(VoxelIndex::new(cell.x), VoxelIndex::new(cell.y), boundary);
    if surface.slab_state(&slab_key) == SlabState::Present {
        return Some(MarchResult {
            kind:   MarchKind::Slab,
            at:     slab_key,
            band:   round_band_for_cell(cross, tuning),
            impact: cross,
        });
    }
    None
}

fn floor_level(z: SimUnit) -> VoxelIndex {
    let floored = (*z).floor();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; the fractional part is gone after floor"
    )]
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    VoxelIndex::new(clamped.clamp(0, i32::from(MAX_LEVELS)))
}

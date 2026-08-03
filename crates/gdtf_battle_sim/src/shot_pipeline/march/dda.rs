//! 3D DDA stepper and per-cell impact tests.

use bevy::prelude::Entity;

use crate::{
    clearance::{Clearance, round_band_for_cell, round_clears_occupant},
    cover::{CoverLedger, HeightBand},
    march::{
        geom::{
            AxisDir, AxisStep, MarchDir, RayParam, VoxelIndex, key_of, key_of_clamped, point_at,
            xy_in_grid, z_in_grid,
        },
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, MAX_LEVELS, SimPos, SimUnit},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

/// Safety cap on march steps.
pub(super) const MAX_STEPS: u32 = (GRID_WIDTH + GRID_HEIGHT + MAX_LEVELS as usize) as u32 * 4;

#[derive(Debug, Clone, Copy)]
struct AxisDda {
    index: VoxelIndex,
    step: AxisStep,
    t_max: RayParam,
    t_delta: RayParam,
}

impl AxisDda {
    fn new(origin: SimUnit, dir: AxisDir, index: VoxelIndex) -> Self {
        if *dir == 0.0 {
            return Self {
                index,
                step: AxisStep::new(0),
                t_max: RayParam::new(f32::INFINITY),
                t_delta: RayParam::new(f32::INFINITY),
            };
        }
        let step = if *dir > 0.0 { 1 } else { -1 };
        let t_delta = (1.0 / *dir).abs();
        #[expect(
            clippy::cast_precision_loss,
            reason = "voxel indices are tiny (0..60 / 0..8); the f32 conversion is exact for this range"
        )]
        let next_boundary = if step > 0 {
            (*index as f32 + 1.0) - *origin
        } else {
            *origin - *index as f32
        };
        let t_max = next_boundary.abs() / (*dir).abs();
        Self {
            index,
            step: AxisStep::new(step),
            t_max: RayParam::new(t_max),
            t_delta: RayParam::new(t_delta),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SteppedAxis {
    X,
    Y,
    Z,
}

/// Test whether the current cell contains an impact (ganger, cover, or occluder).
pub(super) fn impact_at(
    here: CellLevel,
    here_point: SimPos,
    test_band: HeightBand,
    occupancy: &OccupancyGrid,
    cover: &CoverLedger,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<MarchResult> {
    if let (Some(entity), Some(occ_band)) =
        (occupancy.occupant(&here), occupancy.occupant_band(&here))
        && round_clears_occupant(test_band, occ_band) == Clearance::Impacts
        && !is_dead(entity)
    {
        return Some(MarchResult {
            kind: MarchKind::Ganger(entity),
            at: here,
            band: test_band,
            impact: here_point,
        });
    }
    if !*occupancy.is_cover_destroyed(&here)
        && let Some(entry) = cover.peek(&here)
        && !*entry.destroyed
        && round_clears_occupant(test_band, entry.height_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind: MarchKind::Cover(*entry),
            at: here,
            band: test_band,
            impact: here_point,
        });
    }
    if let Some(occluder_band) = occupancy.vision_occluder_at(&here)
        && round_clears_occupant(test_band, occluder_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind: MarchKind::Slab,
            at: here,
            band: test_band,
            impact: here_point,
        });
    }
    None
}

/// Result of one DDA step.
pub(super) enum Step {
    /// Keep marching.
    Continue,
    /// Hit something or left the map.
    Stopped(MarchResult),
}

/// Mutable DDA state while marching.
pub(super) struct MarchState {
    pub(super) vx: VoxelIndex,
    pub(super) vy: VoxelIndex,
    pub(super) vz: VoxelIndex,
    ax: AxisDda,
    ay: AxisDda,
    az: AxisDda,
    pub(super) entry_t: RayParam,
}

impl MarchState {
    pub(super) fn new(
        vx: VoxelIndex,
        vy: VoxelIndex,
        vz: VoxelIndex,
        muzzle: SimPos,
        dir: MarchDir,
    ) -> Self {
        Self {
            vx,
            vy,
            vz,
            ax: AxisDda::new(SimUnit::new(muzzle.x), AxisDir::new(dir.x), vx),
            ay: AxisDda::new(SimUnit::new(muzzle.y), AxisDir::new(dir.y), vy),
            az: AxisDda::new(SimUnit::new(muzzle.z), AxisDir::new(dir.z), vz),
            entry_t: RayParam::new(0.0),
        }
    }

    /// World point where the ray exits the current voxel.
    pub(super) fn exit_point(&self, muzzle: SimPos, dir: MarchDir) -> SimPos {
        let t_exit = RayParam::new((*self.ax.t_max).min(*self.ay.t_max).min(*self.az.t_max));
        point_at(muzzle, dir, t_exit)
    }

    /// Advance one voxel and return Continue or Stopped.
    pub(super) fn advance(
        &mut self,
        muzzle: SimPos,
        dir: MarchDir,
        surface: &SurfaceGrid,
        tuning: &CombatTuning,
        round_band: HeightBand,
    ) -> Step {
        let stepped = step_axis(&self.ax, &self.ay, &self.az);
        let step_t = match stepped {
            SteppedAxis::X => self.ax.t_max,
            SteppedAxis::Y => self.ay.t_max,
            SteppedAxis::Z => self.az.t_max,
        };
        let crossing = point_at(muzzle, dir, step_t);
        self.entry_t = step_t;

        match stepped {
            SteppedAxis::X => {
                self.vx = VoxelIndex::new(*self.vx + *self.ax.step);
                self.ax.index = self.vx;
                self.ax.t_max = RayParam::new(*self.ax.t_max + *self.ax.t_delta);
                if *xy_in_grid(self.vx, self.vy) {
                    Step::Continue
                } else {
                    Step::Stopped(lateral_miss(
                        crossing, self.vx, self.vy, self.vz, round_band,
                    ))
                }
            }
            SteppedAxis::Y => {
                self.vy = VoxelIndex::new(*self.vy + *self.ay.step);
                self.ay.index = self.vy;
                self.ay.t_max = RayParam::new(*self.ay.t_max + *self.ay.t_delta);
                if *xy_in_grid(self.vx, self.vy) {
                    Step::Continue
                } else {
                    Step::Stopped(lateral_miss(
                        crossing, self.vx, self.vy, self.vz, round_band,
                    ))
                }
            }
            SteppedAxis::Z => self.advance_z(crossing, surface, tuning),
        }
    }

    fn advance_z(
        &mut self,
        crossing: SimPos,
        surface: &SurfaceGrid,
        tuning: &CombatTuning,
    ) -> Step {
        let upper_level = self.vz.max(VoxelIndex::new(*self.vz + *self.az.step));
        if *z_in_grid(upper_level) {
            let slab_key = key_of(self.vx, self.vy, upper_level);
            if surface.slab_state(&slab_key) == SlabState::Present {
                return Step::Stopped(MarchResult {
                    kind: MarchKind::Slab,
                    at: slab_key,
                    band: round_band_for_cell(crossing, tuning),
                    impact: crossing,
                });
            }
        }
        self.vz = VoxelIndex::new(*self.vz + *self.az.step);
        self.az.index = self.vz;
        self.az.t_max = RayParam::new(*self.az.t_max + *self.az.t_delta);
        if *self.vz >= i32::from(MAX_LEVELS) {
            Step::Stopped(MarchResult {
                kind: MarchKind::Miss,
                at: key_of_clamped(self.vx, self.vy, self.vz),
                band: round_band_for_cell(crossing, tuning),
                impact: crossing,
            })
        } else if *self.vz < 0 {
            Step::Stopped(MarchResult {
                kind: MarchKind::Ground,
                at: key_of_clamped(self.vx, self.vy, VoxelIndex::new(0)),
                band: HeightBand::Low,
                impact: crossing,
            })
        } else {
            Step::Continue
        }
    }
}

fn lateral_miss(
    impact: SimPos,
    x: VoxelIndex,
    y: VoxelIndex,
    z: VoxelIndex,
    band: HeightBand,
) -> MarchResult {
    MarchResult {
        kind: MarchKind::Miss,
        at: key_of_clamped(x, y, z),
        band,
        impact,
    }
}

fn step_axis(ax: &AxisDda, ay: &AxisDda, az: &AxisDda) -> SteppedAxis {
    if ax.t_max <= ay.t_max && ax.t_max <= az.t_max {
        SteppedAxis::X
    } else if ay.t_max <= az.t_max {
        SteppedAxis::Y
    } else {
        SteppedAxis::Z
    }
}

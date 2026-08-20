//! The cells a round crosses between its muzzle and where it stopped.

use bevy::math::Vec3;

use crate::{
    march::{
        dda::MAX_STEPS,
        geom::{AxisDir, AxisStep, RayParam, VoxelIndex, key_of_clamped},
    },
    metric::{CellLevel, SimPos, SimUnit, cell_center, pos_to_cell},
};

/// Every cell one round passed through, muzzle cell first and impact cell last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundPath(Vec<CellLevel>);

impl RoundPath {
    /// Wrap the cells a round crossed.
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>) -> Self {
        Self(cells)
    }

    /// The cells, muzzle first.
    pub fn cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.0.iter().copied()
    }

    /// How many cells the round passed through.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the round crossed nothing at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the round passed through this cell.
    #[must_use]
    pub fn contains_cell(&self, cell: &CellLevel) -> bool {
        self.0.contains(cell)
    }
}

/// One axis of the voxel walk from the muzzle towards the impact cell.
#[derive(Debug, Clone, Copy)]
struct Axis {
    index:   VoxelIndex,
    step:    AxisStep,
    t_max:   RayParam,
    t_delta: RayParam,
}

impl Axis {
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
        let boundary = if step > 0 {
            (*index as f32 + 1.0) - *origin
        } else {
            *origin - *index as f32
        };
        Self {
            index,
            step: AxisStep::new(step),
            t_max: RayParam::new(boundary.abs() / (*dir).abs()),
            t_delta: RayParam::new((1.0 / *dir).abs()),
        }
    }

    fn advance(&mut self) {
        self.index = VoxelIndex::new((*self.index).saturating_add(*self.step));
        self.t_max = RayParam::new(*self.t_max + *self.t_delta);
    }
}

/// The cells a round crosses flying from `muzzle` to the cell it stopped in.
/// The walk is the voxel traversal the march itself uses.
#[must_use]
pub fn cells_crossed(muzzle: SimPos, impact: CellLevel) -> RoundPath {
    let (start_cell, start_level) = pos_to_cell(muzzle);
    let start = CellLevel::new(start_cell, start_level);
    let mut crossed = vec![start];
    if start == impact {
        return RoundPath::new(crossed);
    }

    let target = cell_center(impact.cell(), impact.level());
    let delta = *target - *muzzle;
    if delta == Vec3::ZERO {
        return RoundPath::new(crossed);
    }
    let direction = delta.normalize_or_zero();
    if direction == Vec3::ZERO {
        return RoundPath::new(crossed);
    }

    let mut x = Axis::new(
        SimUnit::new(muzzle.x),
        AxisDir::new(direction.x),
        VoxelIndex::new(start_cell.x),
    );
    let mut y = Axis::new(
        SimUnit::new(muzzle.y),
        AxisDir::new(direction.y),
        VoxelIndex::new(start_cell.y),
    );
    let mut z = Axis::new(
        SimUnit::new(muzzle.z),
        AxisDir::new(direction.z),
        VoxelIndex::new(i32::from(*start_level)),
    );

    for _ in 0..MAX_STEPS {
        if x.t_max <= y.t_max && x.t_max <= z.t_max {
            x.advance();
        } else if y.t_max <= z.t_max {
            y.advance();
        } else {
            z.advance();
        }
        let here = key_of_clamped(x.index, y.index, z.index);
        if crossed.last() != Some(&here) {
            crossed.push(here);
        }
        if here == impact {
            break;
        }
    }
    RoundPath::new(crossed)
}

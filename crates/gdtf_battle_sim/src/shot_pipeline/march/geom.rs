use bevy::{math::Vec3, prelude::Deref};

use crate::{
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VoxelIndex(i32);

impl VoxelIndex {
        #[must_use]
    pub(super) const fn new(index: i32) -> Self {
        Self(index)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AxisStep(i32);

impl AxisStep {
        #[must_use]
    pub(super) const fn new(step: i32) -> Self {
        Self(step)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct RayParam(f32);

impl RayParam {
        #[must_use]
    pub(super) const fn new(t: f32) -> Self {
        Self(t)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct AxisDir(f32);

impl AxisDir {
        #[must_use]
    pub(super) const fn new(component: f32) -> Self {
        Self(component)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct MarchDir(Vec3);

impl MarchDir {
            #[must_use]
    pub const fn new(dir: Vec3) -> Self {
        Self(dir)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InGrid(bool);

impl InGrid {
        #[must_use]
    pub const fn new(inside: bool) -> Self {
        Self(inside)
    }
}

pub(super) fn xy_in_grid(x: VoxelIndex, y: VoxelIndex) -> InGrid {
    let Ok(ux) = usize::try_from(*x) else {
        return InGrid::new(false);
    };
    let Ok(uy) = usize::try_from(*y) else {
        return InGrid::new(false);
    };
    InGrid::new(ux < GRID_WIDTH && uy < GRID_HEIGHT)
}

pub(super) fn z_in_grid(z: VoxelIndex) -> InGrid {
    InGrid::new(*z >= 0 && *z < i32::from(MAX_LEVELS))
}

pub(super) fn key_of(x: VoxelIndex, y: VoxelIndex, z: VoxelIndex) -> CellLevel {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is checked in 0..MAX_LEVELS before this is called, so the u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(*z as u8);
    CellLevel::new(Cell::new(*x, *y), level)
}

pub(super) fn key_of_clamped(x: VoxelIndex, y: VoxelIndex, z: VoxelIndex) -> CellLevel {
    let clamped = (*z).clamp(0, i32::from(MAX_LEVELS) - 1);
    key_of(x, y, VoxelIndex::new(clamped))
}

pub(super) fn point_at(muzzle: SimPos, dir: MarchDir, t: RayParam) -> SimPos {
    let p = *muzzle + *dir * *t;
    SimPos::new(p.x, p.y, p.z)
}

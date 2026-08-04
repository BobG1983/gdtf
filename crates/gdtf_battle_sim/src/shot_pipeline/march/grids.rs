//! The read-only grids a ray is traced against.

use crate::{cover::CoverLedger, occupancy::OccupancyGrid, surface::SurfaceGrid};

/// Occupancy, surface, and cover grids one march or sight ray is traced against.
#[derive(Debug, Clone, Copy)]
pub struct MarchGrids<'a> {
    /// Who and what stands in each cell.
    pub occupancy: &'a OccupancyGrid,
    /// Floor and slab heights.
    pub surface:   &'a SurfaceGrid,
    /// Cover entries by cell.
    pub cover:     &'a CoverLedger,
}

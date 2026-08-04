//! March a ray through the grid until it hits something or leaves the map.

mod arc;
mod dda;
mod geom;
mod grids;
mod result;
mod vector;

#[cfg(test)]
mod test;

pub use arc::march_arc;
pub use geom::{InGrid, MarchDir};
pub use grids::MarchGrids;
pub use result::{MarchKind, MarchResult};
pub use vector::march_vector;

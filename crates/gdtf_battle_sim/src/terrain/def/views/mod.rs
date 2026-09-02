//! A terrain def's per-view art and the set of views it owes.
mod art;
mod owed;

#[cfg(test)]
mod test;

pub use art::{TerrainView, TerrainViewArt, TerrainViews};
pub use owed::{OwedViews, owed_views, owed_views_for};

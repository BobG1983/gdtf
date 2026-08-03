//! vocabulary — one variant per authorable graphic-role key, the ONE place a
//! mapping, the editor's authorable-picker filter, and the prefab connector
mod vocab;

#[cfg(test)]
mod test;

pub use vocab::TileRole;

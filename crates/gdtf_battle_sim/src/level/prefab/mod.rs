//! - collapses all authored geometry into ONE
mod placement;
mod registry;
mod spec;
mod vocab;

#[cfg(test)]
mod test;

pub use placement::TerrainPlacementEntry;
pub use registry::{Prefab, PrefabKey, PrefabRegistry};
pub use spec::PrefabSpec;
pub use vocab::{PrefabName, SpawnRole};

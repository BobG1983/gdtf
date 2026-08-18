//! Terrain definitions after the legacy per-file authoring types were retired.
mod blocking;
mod definition;
mod kind;
mod registry;
mod uuid;

#[cfg(test)]
mod test;

pub use blocking::{
    Openable, closed_openable_vision_band, derives_path_blocking, derives_vision_occlusion,
    is_openable, los_blocking_to_band, resolved_los_blocking, sim_kind_blocks_path,
    sim_kind_default_los, sim_kind_occludes_vision,
};
pub use definition::{BlocksPathingOverride, TerrainDef, TerrainDisplayName};
pub use kind::{
    LosBlocking, TerrainPresenterKind, TerrainSimKind, TerrainTag, rotated_entry_sides,
};
pub use registry::TerrainDefRegistry;
pub(crate) use uuid::fnv1a64_u128;
pub use uuid::{NilKey, TerrainUuid};

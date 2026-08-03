//! Per-tile terrain ECS entity layer — one entity per authored terrain piece.

/// Component markers and piece kinds.
pub mod components;
/// Lookup index from cell to terrain entity.
pub mod index;

#[cfg(test)]
pub(crate) mod test;

pub use components::{
    BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainPieceKind,
};
pub use index::{TerrainIndex, TerrainIndexKey};

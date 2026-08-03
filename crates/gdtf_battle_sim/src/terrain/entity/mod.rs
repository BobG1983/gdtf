//! The per-tile terrain ECS entity layer — one entity per authored terrain piece
pub mod components;
pub mod index;

#[cfg(test)]
pub(crate) mod test;

pub use components::{
    BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainPieceKind,
};
pub use index::{TerrainIndex, TerrainIndexKey};

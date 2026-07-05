//! The authored value graph: [`PlacedGanger`], [`GangerSpawn`], [`CoverSpawn`],
//! [`SlabSpawn`], [`FloorSpawn`], and the canonical [`Situation`] — the
//! serde-deserializable battlefield the setup is built from.

mod ganger_spawn;
mod piece_spawns;
mod placed_ganger;
mod situation;

pub use ganger_spawn::GangerSpawn;
pub use piece_spawns::{CoverSpawn, FieldSpawn, FloorSpawn, SlabSpawn};
pub use placed_ganger::{PlacedGanger, Placement};
pub use situation::Situation;

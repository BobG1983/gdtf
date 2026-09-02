//! What a destroyed terrain piece leaves behind, and the despawn of the piece it replaces.

mod components;
mod despawn;
mod plan;
mod replace;
mod spawn;
mod writes;

#[cfg(test)]
mod test;

pub use components::{ReplacedPiece, SlabLeftOpen};
pub use despawn::despawn_replaced_piece;
pub use replace::replace_destroyed_piece;
pub use writes::SuccessorWrites;

//! Place a paired DOWN connector when an UP connector is painted.
//! Pair lands as independent terrain, as if both were drawn by hand.
mod pairing;
mod resolve;

#[cfg(test)]
mod tests;

pub use pairing::{PairingOutcome, apply_placement_with_pairing};
pub use resolve::{is_up_connector, resolve_down_counterpart};

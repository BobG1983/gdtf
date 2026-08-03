//! at `(x, y, N)` ALSO places its paired DOWN connector at `(x, y, N+1)`, so authoring a
//! endpoint on storey `N+1`. The author should draw one endpoint and get both. This module is that
//! two independent placed terrains, exactly as if the author had painted both by hand).
mod pairing;
mod resolve;

#[cfg(test)]
mod tests;

pub use pairing::{PairingOutcome, apply_placement_with_pairing};
pub use resolve::{is_up_connector, resolve_down_counterpart};

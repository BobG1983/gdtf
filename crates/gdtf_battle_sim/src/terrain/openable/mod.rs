//! Doors and other openable blockers.

pub mod state;
pub mod toggle;

#[cfg(test)]
mod test;

pub use state::{DoorOpen, OpenState, OpenableBlocking};
pub use toggle::{OpenableTogglePlugin, SetOpenable, apply_openable_toggle};

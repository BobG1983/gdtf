//! The **openable** (door / hatch) layer (GTW-503, child 482c — the FINAL child of the
//! tag-driven-terrain epic GTW-482).
//!
//! Consumes the [`Openable`](crate::terrain::def::TerrainTag::Openable) tag as a toggleable
//! open/closed STATE that flips the GTW-501 / GTW-502 blocking components:
//!
//! - [`OpenState`] — the closed/open state [`Component`](bevy::prelude::Component) attached at
//!   terrain-entity spawn to every openable piece (default
//!   [`Closed`](OpenState::Closed)). [`OpenableBlocking`] — the band a closed door occludes
//!   vision at, recorded so the toggle can re-block without re-reading the def.
//! - [`SetOpenable`] — the toggle MESSAGE (the API GTW-315 will call) + [`apply_openable_toggle`]
//!   — the system that flips the state and ADDS / REMOVES
//!   [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) +
//!   [`BlocksVision`](crate::terrain::entity::BlocksVision) accordingly. [`OpenableTogglePlugin`]
//!   wires both.
//!
//! ## Reuse, not re-plumbing (C4)
//!
//! GTW-503 adds NO projection or recompute system. Adding / removing the GTW-501 / GTW-502
//! components is ALL the toggle does; the existing
//! [`project_path_blocking`](crate::occupancy::project_path_blocking) /
//! [`project_vision_blocking`](crate::occupancy::project_vision_blocking) change-detection
//! re-projects the path + vision surfaces, and
//! [`should_recompute_visibility`](crate::visibility::should_recompute_visibility) re-fires the
//! squad-fog recompute — automatically. See [`toggle`] for the ordering + one-frame settle.

pub mod state;
pub mod toggle;

#[cfg(test)]
mod test;

pub use state::{OpenState, OpenableBlocking};
pub use toggle::{OpenableTogglePlugin, SetOpenable, apply_openable_toggle};

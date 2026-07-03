//! The GTW-564 hot-RON seam: ONE generic handle + config + system triplet for
//! every hot-reloadable single-file RON resource.
//!
//! Wiring-only module. The pieces live in focused submodules: [`mod@handle`]
//! (the one generic [`HotRonHandle`]), [`mod@chain`] (the per-chain
//! [`HotRonChain`] config + the map / fallback hook types), [`mod@systems`]
//! (the kick-off / resolve / redrive triplet encoding the four chain traps
//! exactly once), and [`mod@ext`] (the [`HotRonAppExt`] one-call registration).

mod chain;
mod ext;
mod handle;
mod systems;

pub use chain::{HotRonChain, HotRonFallbackFn, HotRonMapFn, HotRonPath};
pub use ext::HotRonAppExt;
pub use handle::HotRonHandle;
pub(crate) use systems::short_type_name;
pub use systems::{kick_off_hot_ron_resource, redrive_hot_ron_resource, resolve_hot_ron_resource};

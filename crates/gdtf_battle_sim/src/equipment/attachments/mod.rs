//! authoring/resolution/application machinery that RESOLVES a weapon's authored attachment
mod apply;
mod commands;
mod fit;
mod key;
mod registry;
mod slot;
mod spec;

#[cfg(test)]
mod tests;

pub use apply::apply_pending_attachments;
pub use commands::AttachToWeaponExt;
pub use fit::{FitRejection, attachment_fits, resolve_pending_attachments};
pub use key::AttachmentName;
pub use registry::AttachmentRegistry;
pub use slot::{AttachmentSlot, SlotCapacity, WeaponSlots};
pub use spec::AttachmentSpec;

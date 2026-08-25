//! Weapon attachments: slots, registry, fit rules, and apply systems.

mod apply;
mod commands;
mod fit;
mod fitted;
mod key;
mod registry;
mod slot;
mod spec;

#[cfg(test)]
mod tests;

pub use apply::apply_pending_attachments;
pub use commands::AttachToWeaponExt;
pub use fit::{FitRejection, attachment_fits, resolve_pending_attachments};
pub use fitted::FittedAttachments;
pub use key::AttachmentName;
pub use registry::AttachmentRegistry;
pub use slot::{AttachmentSlot, SlotCapacity, WeaponSlots};
pub use spec::AttachmentSpec;

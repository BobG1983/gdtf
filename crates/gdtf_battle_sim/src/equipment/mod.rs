//! The gear a fighter wields/wears: weapons, armor, armor wear, magazines, attachments.

pub mod armor;
pub mod armor_wear;
/// The **weapon-attachment mechanics** (GTW-558) — the registry / spec / key / commands
/// extension / spawn-applier that resolve + apply the attachment-effect palette (which lives
/// under [`crate::effects::attachments`]). See the [`attachments`] module doc.
pub mod attachments;
pub mod magazine;
pub mod weapon;

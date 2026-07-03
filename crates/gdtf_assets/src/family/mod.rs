//! The GTW-570 **content-family** seam: one generic kick-off / resolve /
//! redrive chain turning a folder of loose `.ron` files into a live registry
//! [`Resource`](bevy::prelude::Resource).
//!
//! Before GTW-570 every FOLDER-loaded content family (weapons, melee weapons,
//! armor, fields, gangs, terrain defs, theme defs) hand-stamped the same
//! ~330-line resolve module, poll branch, handle newtypes, and redrive clone
//! across four crates. The shared behaviors now live here exactly once — see
//! [`ContentFamily`] for the trait contract and the add-one-family recipe, and
//! [`ContentFamilyAppExt`] for the one-call registration.

mod def;
mod ext;
mod handle;
mod systems;

pub use def::{ContentFamily, ContentFileStem};
pub use ext::ContentFamilyAppExt;
pub use handle::ContentFolderHandle;
pub use systems::{kick_off_content_family, redrive_content_family, resolve_content_family};

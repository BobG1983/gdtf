//! The GTW-570 **content-family** machinery: one generic kick-off / resolve /
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
mod report;
mod salvage;
mod systems;

pub use def::{ContentFamily, ContentFileStem};
pub use ext::ContentFamilyAppExt;
pub use handle::ContentFolderHandle;
pub use report::{
    ContentChecksComplete, ContentFinding, ContentIntegrityReport, ContentValidationAppExt,
    ContentValidationDone, ContentValidationSet, FindingDetail, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceKeyScheme, mark_content_checks_complete,
    publish_content_integrity_report,
};
pub use salvage::{
    MalformedMember, RonFolderSalvage, RonSalvagePoll, SalvageFolder, SalvageMemberPath,
    SalvagedMember, begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
    salvage_members_for_rebuild,
};
pub use systems::{kick_off_content_family, redrive_content_family, resolve_content_family};

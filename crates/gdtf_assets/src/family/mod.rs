//! Folder-based content families: load, salvage, validate, hot-rebuild registries.

mod def;
mod ext;
mod handle;
mod report;
mod salvage;
mod source;
mod systems;

pub use def::{ContentFamily, ContentFileStem, ContentMemberKey};
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
pub use source::{ContentSourcePath, ContentSourcePaths, PublishedFamily};
pub use systems::{kick_off_content_family, redrive_content_family, resolve_content_family};

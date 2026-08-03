//! RON asset loading, hot-reload resources, content families, and save helpers.

mod asset;
mod error;
mod ext;
mod family;
mod hot;
mod loader;
mod save;
mod workspace;

pub use asset::RonAsset;
pub use error::{ReadError, RonDeError, RonLoadError};
pub use ext::RonAssetAppExt;
pub use family::{
    ContentChecksComplete, ContentFamily, ContentFamilyAppExt, ContentFileStem, ContentFinding,
    ContentFolderHandle, ContentIntegrityReport, ContentValidationAppExt, ContentValidationDone,
    ContentValidationSet, FindingDetail, FindingFamily, FindingReferrer, FindingTarget,
    MalformedMember, ReferenceKeyScheme, RonFolderSalvage, RonSalvagePoll, SalvageFolder,
    SalvageMemberPath, SalvagedMember, begin_ron_folder_salvage, kick_off_content_family,
    mark_content_checks_complete, poll_ron_folder_salvage, publish_content_integrity_report,
    redrive_content_family, report_malformed_members, resolve_content_family,
    salvage_members_for_rebuild,
};
pub use hot::{
    HotRonAppExt, HotRonChain, HotRonFallbackFn, HotRonHandle, HotRonMapFn, HotRonPath,
    kick_off_hot_ron_resource, redrive_hot_ron_resource, resolve_hot_ron_resource,
};
pub use loader::RonAssetLoader;
#[cfg(debug_assertions)]
pub use save::write_ron_pretty;
pub use save::{FileStem, RonSaveError, sanitize_file_stem, serialize_ron_pretty};
pub use workspace::WORKSPACE_ASSETS_ROOT;

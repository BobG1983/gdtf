//! RON asset loading, hot-reload resources, save helpers, and the workspace-root search.

mod asset;
mod error;
mod ext;
mod hot;
mod loader;
mod save;
mod workspace;

pub use asset::RonAsset;
pub use error::{ReadError, RonDeError, RonLoadError};
pub use ext::RonAssetAppExt;
pub use hot::{
    HotRonAppExt, HotRonChain, HotRonFallbackFn, HotRonHandle, HotRonMapFn, HotRonPath,
    HotRonResolved, kick_off_hot_ron_resource, redrive_hot_ron_resource, resolve_hot_ron_resource,
    short_type_name,
};
pub use loader::RonAssetLoader;
pub use save::{
    FileStem, RonSaveError, sanitize_file_stem, serialize_ron_pretty, write_ron_pretty,
};
pub use workspace::{workspace_assets_root, workspace_root, workspace_root_from};

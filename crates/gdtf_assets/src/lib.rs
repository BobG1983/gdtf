//! Loose-file RON asset loading for GDTF.
//!
//! GDTF ships its data-driven content (the UI theme, and later content tables)
//! as **loose `.ron` files under the workspace-root `assets/` directory** (ADR
//! 0003; embedding is deferred to packaging). This crate provides the one piece
//! every such file needs: a generic Bevy
//! [`AssetLoader`](bevy::asset::AssetLoader) that reads a `.ron` file's bytes
//! and `serde`-deserializes them into a typed payload `T`.
//!
//! # The pieces
//!
//! - [`RonAsset<T>`] — a Bevy [`Asset`](bevy::asset::Asset) newtype wrapping a
//!   deserialized `T`. It
//!   `Deref`s to `T`, so a consumer reads the payload directly off the loaded
//!   asset. `T` is any `serde`-`Deserialize` + `TypePath` value; the loader is
//!   generic over it, so one loader serves every RON content type.
//! - [`RonAssetLoader<T>`] — the [`AssetLoader`](bevy::asset::AssetLoader)
//!   impl. It reads the file bytes
//!   and `ron::de::from_bytes`-deserializes them into `T`, wrapping the result
//!   in [`RonAsset`]. A malformed file surfaces as a **typed** [`RonLoadError`]
//!   (which Bevy records as a failed load), never a panic or `unwrap`.
//! - [`RonLoadError`] — the loader's typed error: either the bytes could not be
//!   read, or the RON did not deserialize into `T`.
//! - [`RonAssetAppExt`] — a one-call registration extension: `app
//!   .init_ron_asset::<T>()` registers `Assets<RonAsset<T>>` and the loader for
//!   `T` in a single step.
//! - [`HotRonAppExt`] (GTW-564) — the HOT-RELOADABLE layer on top: one call
//!   registers a whole kick-off / resolve / redrive chain turning a loose
//!   `.ron` into a live runtime [`Resource`](bevy::prelude::Resource), keyed by
//!   the one generic [`HotRonHandle`] and configured by a per-chain
//!   [`HotRonChain`].
//! - [`ContentFamily`] + [`ContentFamilyAppExt`] (GTW-570) — the FOLDER
//!   counterpart: one marker impl + one
//!   [`register_content_family`](ContentFamilyAppExt::register_content_family)
//!   call turns a whole folder of `.ron` files into a live registry
//!   [`Resource`](bevy::prelude::Resource), hot-reloading on member edits,
//!   keyed by the one generic persistent [`ContentFolderHandle`].
//! - [`sanitize_file_stem`] / [`FileStem`] + [`serialize_ron_pretty`] /
//!   `write_ron_pretty` / [`RonSaveError`] (GTW-577) — the shared RON **save**
//!   seam: the ONE file-stem slug policy and the ONE serialize → mkdir → write
//!   chain (dev-only) every editor-side saver delegates to.
//! - [`WORKSPACE_ASSETS_ROOT`] (GTW-634) — the ONE owning definition of the
//!   workspace `assets/` root every host's `AssetPlugin` and every editor-side
//!   saver share, so read and write roots can never drift apart.
//!
//! # Where the asset source root is
//!
//! The Bevy file [`AssetReader`](bevy::asset::io::AssetReader) resolves loose
//! paths against a base directory chosen by `bevy_asset` as: the `BEVY_ASSET_ROOT`
//! env var if set, else `CARGO_MANIFEST_DIR` if set, else the running
//! executable's directory. Under `cargo run -p <bin>` / `cargo test` that
//! manifest dir is the *binary / test crate*, not the repo root — so the hosts
//! (`GdtfApp`, the map editor's `MapEditorApp`) and the test harness
//! ([`GdtfUiTestAppBuilder`](../gdtf_test_utils/index.html)) all point
//! [`AssetPlugin::file_path`](bevy::asset::AssetPlugin::file_path) explicitly at
//! the workspace-root `assets/` directory, so a path that loads in a test loads
//! in the app. The hosts import that root's ONE spelling from here
//! ([`WORKSPACE_ASSETS_ROOT`]).
//!
//! This crate does **not** configure the asset source root itself — it loads
//! whatever the host app's `AssetPlugin` is pointed at (it only OWNS the
//! spelling hosts configure with) — so it stays a pure, reusable leaf that both
//! GTW-56 (`Load`) and GTW-42 (content) can depend on.

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

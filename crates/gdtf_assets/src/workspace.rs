//! The ONE owning definition of the workspace `assets/` root (GTW-634 C1).
//!
//! Before GTW-634 this constant was re-spelled SIX times (both app wrappers,
//! the three editor-form savers, and the gang-editor saver), so the GTW-562
//! change-class — move an authored-content root — was a multi-site sweep the
//! compiler could not force. It now exists exactly once, here, beside the
//! shared save seam ([`sanitize_file_stem`](crate::sanitize_file_stem) /
//! `write_ron_pretty`): every host's `AssetPlugin.file_path` and every
//! editor-side saver import the SAME string, so read and write roots are
//! byte-identical by construction.

/// Absolute path to the workspace-root `assets/` directory (ADR 0003) — the
/// ONE spelling every host and saver shares.
///
/// Bevy's default file [`AssetReader`](bevy::asset::io::AssetReader) base path
/// is **not** the working directory — `bevy_asset`'s `get_base_path` reads
/// `BEVY_ASSET_ROOT`, else the runtime `CARGO_MANIFEST_DIR`, else the
/// executable's directory. Under `cargo run -p <bin>` that manifest dir is the
/// *binary package*, so the default would resolve `assets/...` under
/// `bins/<bin>/assets`, not the repo root. Hosts therefore point
/// [`AssetPlugin::file_path`](bevy::asset::AssetPlugin::file_path) at THIS
/// constant, and the editor-side savers resolve their write paths under it —
/// one owner, so a saved file lands exactly where the loaders read.
///
/// Computed at compile time relative to THIS crate's manifest
/// (`crates/gdtf_assets` → up two levels → `assets`). Every workspace crate
/// lives at `crates/<name>`, so the resolved directory is identical to what
/// each deleted per-crate copy resolved to; the string is lexical (contains
/// `../..`), which Bevy's reader and `std::fs` both accept.
pub const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

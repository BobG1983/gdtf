//! The shared RON **save helpers** (GTW-577): the one file-stem sanitizer and the one
//! serialize → mkdir → write chain every editor-side saver delegates to.
//!
//! Before GTW-577 the prefab / terrain / theme / gang savers each carried an identical
//! slug filter and an identical `to_string_pretty` → `create_dir_all` → `fs::write`
//! chain. Both halves now exist ONCE, here:
//!
//! - [`sanitize_file_stem`] → [`FileStem`] — the ONE slug policy a display name folds
//!   through before it becomes a file name (so a path-hostile name can never reach the
//!   filesystem raw).
//! - [`serialize_ron_pretty`] / [`write_ron_pretty`] + [`RonSaveError`] — the ONE
//!   pretty-RON serialize and the ONE dev-only disk-write chain, with the shared
//!   serialize-failed / write-failed error the per-form save errors wrap.
//!
//! Per-form concerns stay per-form (the GTW-577 P9 bound): domain-validation variants
//! (`EmptyName` / `IllegalCell` / …), path layout, and the projection into each schema
//! all live with their form — only the duplicated tails collapsed here.

mod stem;
mod write;

pub use stem::{FileStem, sanitize_file_stem};
#[cfg(debug_assertions)]
pub use write::write_ron_pretty;
pub use write::{RonSaveError, serialize_ron_pretty};

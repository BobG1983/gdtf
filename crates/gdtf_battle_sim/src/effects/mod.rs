//! **Authorable effect palettes** (GTW-558, child of GTW-551 → GTW-17) — the families of
//! conceptually-isolated, data-authored EFFECTS the sim can apply to an entity.
//!
//! Each family is a sub-module holding a closed serde **vocabulary** (the on-disk RON
//! enum), a shared **apply trait** whose one method IS an effect's behaviour, and ONE
//! self-contained file per effect (its magnitude, its isolated `ApplyX` type, its `impl`,
//! its unit test). The mechanics that INVOKE a palette live elsewhere (e.g. weapon
//! attachments' registry / commands extension / spawn-applier under
//! [`equipment::attachments`](crate::equipment::attachments)); this module owns only the
//! effect palettes themselves, so the dependency runs one way — mechanics depend on the
//! palette, never the reverse.
//!
//! ## The discipline
//!
//! Adding a new effect to a family = ONE new per-effect file + ONE enum variant + ONE
//! delegation arm (in the family's `effect.rs`) + ONE `mod` line (in the family's `mod.rs`).
//! No central logic `match`, no folder-fn, no authoring step scattered across the codebase.
//! The mechanics NEVER match on the effect enum — they invoke the shared apply trait
//! generically.

/// The **weapon-attachment effect palette** — the closed
/// [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect) vocabulary + one
/// isolated `ApplyX` behaviour per effect (GTW-558; the palette that SUPERSEDES the GTW-542
/// `AttachTag` model). The mechanics that resolve + apply it live under
/// [`equipment::attachments`](crate::equipment::attachments).
pub mod attachments;

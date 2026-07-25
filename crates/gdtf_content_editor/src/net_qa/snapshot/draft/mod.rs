//! The DRAFT topic — the active authoring mode's in-progress draft, as named fields
//! (GTW-805).
//!
//! Ten authoring modes edit ten unrelated record shapes, so each gets its own file: what a
//! TERRAIN draft shows changes when the terrain schema changes, and nothing else does.
//! [`model`] holds the one [`EditorDraftModel`](model::EditorDraftModel) read of all of
//! them, and [`view`] turns the active mode's fields into the wire snapshot.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`fields`] — the shared field-row builders.
//! - [`model`] — the [`SystemParam`](bevy::ecs::system::SystemParam) reading every draft.
//! - [`view`] — the active mode's draft snapshot.
//! - [`terrain`] / [`theme`] / [`prefab`] / [`gang`] / [`armor`] / [`injury`] / [`sprite`] /
//!   [`attachment`] / [`weapon`] / [`melee`] — one per authoring mode.

mod armor;
mod attachment;
mod fields;
mod gang;
mod injury;
mod melee;
mod model;
mod prefab;
mod sprite;
mod terrain;
mod theme;
mod view;
mod weapon;

pub(in crate::net_qa) use model::EditorDraftModel;
pub(in crate::net_qa) use view::draft_view;

//! The data-driven keybind table (GTW-225 / GTW-48 S8): the FIRST keybind `.ron`
//! in the repo and its loader.
//!
//! Every bound act names a key in a loose, per-line-commented
//! `assets/core_tuning/keybinds.tuning.ron`, loaded through the generic GTW-136
//! [`RonAsset<T>`](gdtf_assets::RonAsset) path (the `character_roles`
//! precedent) and resolved into a resident [`Keybinds`] resource. The systems read
//! the resolved [`KeyCode`](bevy::prelude::KeyCode) off that resource — there is NO
//! hardcoded `KeyCode` literal in any system.
//!
//! # Why an authored [`BoundKey`] vocabulary instead of a bare `KeyCode`
//!
//! Bevy's [`KeyCode`](bevy::prelude::KeyCode) only derives `serde::Deserialize`
//! under its `serialize` feature, which is NOT in the workspace's default Bevy
//! feature set (enabling it fans out a heavy recompile across the whole engine). So
//! the authored leaf is a named domain [`BoundKey`] enum — a small, explicit key
//! vocabulary that `Deserialize`s from a RON name and
//! [resolves to](BoundKey::key_code) a [`KeyCode`](bevy::prelude::KeyCode). This is
//! the no-bare-types-aligned shape (an authored key is a named domain value, not a
//! raw framework code) AND it keeps the binding data-driven: the one
//! [`BoundKey`]→`KeyCode` mapping is a typed translation, and every system reads the
//! resolved [`KeyCode`](bevy::prelude::KeyCode) from [`Keybinds`], never a literal.

mod table;

#[cfg(test)]
mod test;

pub(crate) use table::register_keybinds_hot_ron;
pub use table::{BoundKey, Keybinds};

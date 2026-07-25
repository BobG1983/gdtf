//! The INJURIES family — the bespoke one-folder → two-resources content family
//! (a declared GTW-570 machinery EXCLUSION), shared by BOTH hosts (GTW-654).
//!
//! One recursive folder ([`INJURIES_FOLDER`]) carries TWO asset types — the
//! per-injury `<category>/*.injury.ron` defs and the per-category
//! `weighting/*.weighting.ron` tables — resolved into TWO resources
//! ([`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) +
//! [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables)), so there is no
//! `ContentFamily` impl to hang the folder / extensions / build on. Its
//! host-agnostic pieces therefore live HERE (the GTW-630 `validate` precedent:
//! logic both hosts run lives in the glue crate; each host keeps only its own
//! thin registration/systems):
//!
//! - `layout` — the one-owner folder / extension / per-category-directory
//!   spellings ([`INJURIES_FOLDER`] / [`category_dir`] and friends, GTW-634 C4).
//! - `build` — [`build_injury_data`], the ONE folder-walk → registry+tables
//!   builder both hosts' resolve AND redrive delegate to (GTW-437).
//! - `keying` — the stem→key rule and the organizational subfolder audit.
//! - `salvage` — the GTW-582 per-file salvage fold ([`begin_injuries_salvage`] /
//!   [`settle_injuries_salvage`]: one salvage per asset type, settled atomically
//!   into both resources).

mod build;
mod keying;
mod layout;
mod salvage;

pub use build::build_injury_data;
pub use layout::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
    category_dir, weighting_context_infix,
};
pub use salvage::{begin_injuries_salvage, settle_injuries_salvage};

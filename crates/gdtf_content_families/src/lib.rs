//! The GDTF **content-family marker impls** (GTW-570) — one tiny
//! [`ContentFamily`](gdtf_assets::ContentFamily) impl per folder-loaded
//! content family, binding the generic folder→registry seam in `gdtf_assets`
//! to the sim-owned registry types.
//!
//! # Why a separate glue crate
//!
//! `gdtf_assets` is a deliberate LEAF (bevy/ron/serde only) and cannot name
//! the sim's registry types; `gdtf_app` cannot host the impls either, because
//! the content editor needs the SAME families without depending on the game
//! app. This crate is the seam both hosts share: a host registers a family
//! with one line —
//! `app.register_content_family::<WeaponsFamily>()`
//! (see [`ContentFamilyAppExt`](gdtf_assets::ContentFamilyAppExt)) — and the
//! generic chain does the rest.
//!
//! # The seven shipped families
//!
//! Stem-keyed (key = file stem with the dedicated infix stripped):
//! [`WeaponsFamily`], [`MeleeWeaponsFamily`], [`ArmorFamily`],
//! [`FieldsFamily`], [`GangsFamily`]. Payload-keyed (key = the UUID inside the
//! def; the terrain tree is a MIXED folder both walk with the unconditional
//! `TypeId` filter): [`TerrainDefsFamily`], [`ThemeDefsFamily`].

mod armor;
mod fields;
mod gangs;
mod melee_weapons;
mod terrain_defs;
mod theme_defs;
mod weapons;

pub use armor::ArmorFamily;
pub use fields::FieldsFamily;
pub use gangs::GangsFamily;
pub use melee_weapons::MeleeWeaponsFamily;
pub use terrain_defs::TerrainDefsFamily;
pub use theme_defs::ThemeDefsFamily;
pub use weapons::WeaponsFamily;

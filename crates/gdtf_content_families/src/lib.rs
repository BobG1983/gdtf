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
//! # The nine shipped families
//!
//! Stem-keyed (key = file stem with the dedicated infix stripped):
//! [`WeaponsFamily`], [`MeleeWeaponsFamily`], [`ArmorFamily`],
//! [`FieldsFamily`], [`GangsFamily`], [`AttachmentsFamily`],
//! [`SpriteDefsFamily`]. Payload-keyed (key = the UUID inside the def; the
//! terrain tree is a MIXED folder both walk with the unconditional `TypeId`
//! filter): [`TerrainDefsFamily`], [`ThemeDefsFamily`].
//!
//! The [`sprites`] family (GTW-663) is the one family whose `Spec`/`Registry`
//! live IN this crate rather than the sim: the sprite-def model is
//! PRESENTATION data the render-free sim cannot own (see the module doc).
//!
//! Beside the family impls, [`validate`] hosts the HOST-AGNOSTIC per-edge
//! reference checks of the GTW-582 unified dangling-reference contract
//! (GTW-630) — the same one-crate seam logic, shared by both hosts.
//!
//! # The two bespoke families (GTW-634 / GTW-654)
//!
//! The two declared GTW-570 seam EXCLUSIONS — [`prefabs`] (a nested
//! `<theme>/<size>/` tree into a bucketed multimap) and [`injuries`] (one
//! folder, two asset types, two resources) — have no `ContentFamily` impl to
//! carry a `FOLDER` / `EXTENSION`, so their folder + extension consts are
//! declared ONCE in the sibling [`prefabs`] / [`injuries`] modules here.
//! Both hosts import them (the game's bespoke Load chain reads with them; the
//! map editor's savers write with them), so a write-side spelling can
//! never drift from the loader's read (the GTW-621 gang-extension bug class).
//! Since GTW-654 the [`injuries`] module ALSO hosts the family's host-agnostic
//! folder-walk builder + per-file salvage fold (the GTW-630 `validate`
//! precedent), shared by the game's Load resolve and the editor's Load pass.

mod armor;
mod attachments;
mod fields;
mod gangs;
pub mod injuries;
mod melee_weapons;
pub mod prefabs;
pub mod sprites;
mod terrain_defs;
mod theme_defs;
pub mod validate;
mod weapons;

pub use armor::ArmorFamily;
pub use attachments::AttachmentsFamily;
pub use fields::FieldsFamily;
pub use gangs::GangsFamily;
pub use melee_weapons::MeleeWeaponsFamily;
pub use sprites::SpriteDefsFamily;
pub use terrain_defs::TerrainDefsFamily;
pub use theme_defs::ThemeDefsFamily;
pub use weapons::WeaponsFamily;

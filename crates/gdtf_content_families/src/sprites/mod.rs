//! The **sprite-defs content family** (GTW-663) — the per-file
//! `.spritedef.ron` sprite catalog under `content/sprites/` and the
//! [`SpriteDefRegistry`] it resolves into.
//!
//! THE RULING (user, 2026-07-07, on GTW-600): a sprite definition is a real
//! content-family member — `{name = file stem; source: image path OR
//! {sheet, rect}; anchor: (x, y) ground-contact/pivot; optional facings map
//! (the closed 4-facing enum); optional animation {fps, frames}}` — and a
//! terrain def's `graphic_name` is a FOREIGN KEY by name into this registry,
//! validated by the [`validate`](crate::validate) reference-integrity edge.
//! The presenter's `TileRole` stays the closed renderer vocabulary; consuming
//! these defs in the renderer is GTW-665, the editor authoring mode is
//! GTW-664, restamping shipped content is GTW-666.
//!
//! CRATE PLACEMENT: unlike every other family (whose `Spec`/`Registry` the
//! sim owns), the sprite-def model is PRESENTATION data — the render-free
//! `gdtf_battle_sim` cannot own it, and making this glue crate depend on
//! `gdtf_battle_presenter` would invert the one-way graph. So the spec types
//! live HERE, beside the family glue; the presenter gains the (one-way)
//! `presenter → families` read edge when GTW-665 consumes the registry.
//!
//! NAMING CAUTION: `assets/sprites/*.spritedef.ron` ALREADY exists as the OLD
//! role-table format (the presenter's `TileRoles` / character / effect role
//! tables — single-file typed loads). This family lives under
//! `assets/content/sprites/` (the content root, like every family); the old
//! tables keep working untouched until GTW-665 supersedes them. The two never
//! collide at the loader: a typed load resolves by ASSET TYPE first
//! (`bevy_asset` `loaders.rs::find` — a single-candidate type match returns
//! before extension dispatch), and the untyped folder walk dispatches on the
//! full `spritedef.ron` extension, which only this family's loader claims.

mod def;
mod family;
mod registry;
mod source;

#[cfg(test)]
mod test;

pub use def::{SpriteAnchor, SpriteAnimation, SpriteDef, SpriteFacing, SpriteFacings, SpriteFps};
pub use family::SpriteDefsFamily;
pub use registry::{SpriteDefRegistry, SpriteName};
pub use source::{SpriteImagePath, SpritePx, SpriteRect, SpriteSource};

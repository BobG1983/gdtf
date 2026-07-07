//! The editor's **per-def terrain-graphic resolution** (GTW-495 / GTW-665) — resolve a
//! [`TerrainUuid`] to the SPRITE DEF its graphic draws, exactly the way the
//! PRESENTER does.
//!
//! The legacy editor read a tile's atlas index straight off a catalog tile's own index field.
//! The UUID-keyed model carries NO per-def atlas index: a
//! [`TerrainDef`]'s [`presenter_kind`](TerrainDef::presenter_kind) names a GRAPHIC KEY
//! ([`TerrainGraphicKey`]) — since GTW-663/665 a FOREIGN KEY by name into the
//! [`SpriteDefRegistry`] — which the presenter resolves to a texture + rect + anchor
//! through [`resolve_sprite`] (the ONE resolution). The editor consumes the SAME
//! resolution here so its palette / preview sprites match the battlescape art —
//! never a parallel vocabulary or a second table.

use gdtf_battle_presenter::resolve_sprite;
use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainUuid},
    piece::TerrainGraphicKey,
};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefRegistry};

/// The graphic KEY a terrain definition draws with — the
/// [`graphic_name`](TerrainGraphicKey) carried on every [`TerrainPresenterKind`] variant
/// (`Wall` / `Cover` / `Slab` / `Emplacement`), uniformly extracted (GTW-495 / GTW-543).
///
/// The presenter half names the sprite name for ALL kinds; this returns it so the editor
/// can hand it to the presenter's [`resolve_sprite`] the way the battle draw does.
#[must_use]
pub(crate) const fn graphic_key(def: &TerrainDef) -> &TerrainGraphicKey {
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

/// Resolve a [`TerrainUuid`] to its [`SpriteDef`] — the sprite a palette row /
/// preview cell fill draws (GTW-495 / GTW-665), resolved THE WAY THE PRESENTER DOES.
///
/// Looks the [`TerrainUuid`] up in the [`TerrainDefRegistry`], reads its
/// [`presenter_kind`](TerrainDef::presenter_kind)'s [`graphic_name`](TerrainGraphicKey), and
/// resolves that name through the presenter's [`resolve_sprite`] over the GTW-663
/// [`SpriteDefRegistry`] — exactly the battle draw's resolution. Returns [`None`]
/// if the key names no registered terrain def, or its graphic name resolves no sprite def
/// (the consumer then draws its LOUD missing fallback — the C4 magenta precedent —
/// rather than panicking or vanishing).
#[must_use]
pub(crate) fn terrain_sprite_def<'a>(
    registry: &TerrainDefRegistry,
    sprites: &'a SpriteDefRegistry,
    key: &TerrainUuid,
) -> Option<&'a SpriteDef> {
    let def = registry.def(key)?;
    resolve_sprite(sprites, graphic_key(def))
}

//! The editor's **per-def terrain-graphic resolution** (GTW-495) — resolve a
//! [`TerrainUuid`] to the terrain-sheet atlas index its sprite draws, exactly the way the
//! PRESENTER does.
//!
//! The legacy editor read a tile's atlas index straight off a catalog tile's own index field.
//! The NEW UUID-keyed model carries NO per-def atlas index: a
//! [`TerrainDef`]'s [`presenter_kind`](TerrainDef::presenter_kind) names a GRAPHIC ROLE KEY
//! ([`TerrainGraphicKey`]), which the presenter resolves to an atlas index through its
//! data-driven [`TileRoles`] table ([`TileRoles::index_for_key`], the GTW-493 seam). The editor
//! mirrors that resolution here so its palette / canvas sprites match the battlescape art —
//! reusing the presenter's role table rather than inventing a parallel vocabulary.

use gdtf_battle_presenter::{TileIndex, TileRoles};
use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainUuid},
    piece::TerrainGraphicKey,
};

/// The graphic-role KEY a terrain definition draws with — the
/// [`graphic_name`](TerrainGraphicKey) carried on every [`TerrainPresenterKind`] variant
/// (`Wall` / `Cover` / `Slab` / `Emplacement`), uniformly extracted (GTW-495 / GTW-543).
///
/// The presenter half names the role key for ALL kinds; this returns it so the editor can hand
/// it to [`TileRoles::index_for_key`] the way the presenter's draw does.
#[must_use]
pub(crate) const fn graphic_key(def: &TerrainDef) -> &TerrainGraphicKey {
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

/// Resolve a [`TerrainUuid`] to its terrain-sheet atlas index — the index a palette row /
/// canvas cell fill sprite shows (GTW-495), resolved THE WAY THE PRESENTER DOES.
///
/// Looks the [`TerrainUuid`] up in the [`TerrainDefRegistry`], reads its
/// [`presenter_kind`](TerrainDef::presenter_kind)'s [`graphic_name`](TerrainGraphicKey), and
/// resolves that role key through the presenter's [`TileRoles`] table
/// ([`TileRoles::index_for_key`]) — exactly the GTW-493 presenter resolution. Returns [`None`]
/// if the key names no registered def, or its graphic role is out of the [`TileRoles`]
/// vocabulary (the cell then falls back to the theme default-floor fill rather than panicking).
#[must_use]
pub(crate) fn terrain_atlas_index(
    registry: &TerrainDefRegistry,
    roles: &TileRoles,
    key: &TerrainUuid,
) -> Option<TileIndex> {
    let def = registry.def(key)?;
    roles.index_for_key(graphic_key(def))
}

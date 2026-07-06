//! Def→class recognition for the vertical placement rules (GTW-430; UUID-swept in GTW-495) —
//! registry-only: [`classify`] / [`names_a_ladder`] change when the tile taxonomy changes, never
//! with the map.

use gdtf_battle_sim::{
    level::ThemeUuid,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

/// The editor's classification of a terrain definition for the vertical placement rules
/// (GTW-430; swept to the UUID model in GTW-495).
///
/// A named domain enum (no-bare-types: a tile's placement class is a domain value, not a bare
/// discriminant). Derived from the terrain registry by [`classify`]: [`Slab`](EditorTileClass::Slab)
/// from the sim's [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab), [`Ladder`](EditorTileClass::Ladder) from the
/// ladder-naming convention ([`names_a_ladder`] over the def's display name), and
/// [`Other`](EditorTileClass::Other) for every tile the vertical rules do not constrain (walls,
/// cover).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTileClass {
    /// A floor/roof slab — seals a z-boundary (the sim's [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab)).
    Slab,
    /// A ladder — a vertical link between storeys (recognised by name; the terrain model has no
    /// ladder kind — see the module docs).
    Ladder,
    /// Any tile the vertical placement rules do not constrain (wall / cover).
    Other,
}

/// Whether a terrain's display name reads as a LADDER — the data-driven ladder recognition
/// (GTW-430; swept to read the def's display name in GTW-495).
///
/// The unified terrain model has no ladder *kind* ([`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) is WALL/COVER/SLAB/EMPLACEMENT), and
/// ladders are a sim vertical-link concept, not a terrain piece. So the editor recognises a ladder
/// by NAME: a display name containing `"ladder"` (case-insensitive). This is data-driven (any theme
/// that adds a ladder-named terrain is recognised, not bound to one specific UUID) and consistent
/// with the sim's `LinkKind::Ladder` vocabulary.
#[must_use]
pub fn names_a_ladder(text: &str) -> bool {
    text.to_ascii_lowercase().contains("ladder")
}

/// Classify a terrain definition (looked up by `key` in the [`TerrainDefRegistry`]) into its
/// [`EditorTileClass`] for the vertical placement rules (GTW-430; UUID-keyed in GTW-495).
///
/// Semantics-driven off the terrain registry: the slab class comes from the sim's
/// [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab); the ladder class from the [`names_a_ladder`] convention over the def's
/// display name. A tile the registry does not know (a stale key after a theme switch) is
/// [`Other`](EditorTileClass::Other) — the conservative class the vertical rules never constrain.
/// The `theme` parameter is retained for the shared predicate signature even though the UUID-keyed
/// registry resolves a terrain without it (a UUID is globally unique).
#[must_use]
pub fn classify(
    registry: &TerrainDefRegistry,
    _theme: ThemeUuid,
    key: &TerrainUuid,
) -> EditorTileClass {
    let Some(def) = registry.def(key) else {
        return EditorTileClass::Other;
    };
    if names_a_ladder(&def.display_name) {
        return EditorTileClass::Ladder;
    }
    // A kind-IDENTITY decision (no per-variant payload), so it classifies over the canonical
    // `TerrainPieceKind` projection (GTW-574 C2) — exhaustive, no wildcard.
    match def.sim_kind.kind() {
        TerrainPieceKind::Slab => EditorTileClass::Slab,
        // GTW-543: an emplacement is a same-level structure (like Wall/Cover) placed on the
        // canvas, not a slab z-boundary — it classifies as Other.
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => {
            EditorTileClass::Other
        }
    }
}

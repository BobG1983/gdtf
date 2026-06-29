//! Pure **projection and serialization** functions — convert the in-memory [`EditorMap`] into
//! the canonical GTW-418 [`PrefabSpec`] schema and serialize it to RON.
//!
//! No Bevy systems here — only pure data-transformation functions and their helpers. All
//! filesystem I/O lives in [`super::systems`].

use std::path::{Path, PathBuf};

use gdtf_battle_sim::{
    Cell,
    level::{
        CatalogTileKind, EdgeOpening, GridSize, LevelTheme, PrefabPiece, PrefabSpec,
        ThemeCatalogRegistry, TileKey,
    },
    metric::{CellLevel, Level},
    terrain::piece::TerrainName,
    vertical::{LinkKind, VerticalLink},
};

use super::types::{
    MAPS_SUBDIR, PREFAB_EXTENSION, SAVED_SPAWN_ROLE, SavePrefabError, WORKSPACE_ASSETS_ROOT,
    size_dir, theme_dir,
};
use crate::{
    EditorMap,
    placement::{
        EditorTileClass, PlacementVerdict, ProposedPlacement, classify, evaluate_placement,
    },
    session::MapEditorSession,
};

/// Sanitize the entered prefab name into a file-name STEM — trimmed, lowercased, spaces →
/// underscores, and any character outside `[a-z0-9_]` dropped (GTW-432).
///
/// The author types a free-form name into the GTW-411 text field; this folds it to the stem
/// convention the shipped prefab files use (`entry_room`). Returns the empty string for a
/// name that sanitizes to nothing, which the caller treats as [`SavePrefabError::EmptyName`].
#[must_use]
pub(super) fn sanitize_name(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The full on-disk PATH a saved prefab is written to:
/// `<workspace assets>/content/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-432).
///
/// Pure (no IO) so a test can assert the resolved location without writing anything. The `theme`
/// / `size` come from the prefab being authored; `stem` is the sanitized prefab name.
#[must_use]
pub(crate) fn prefab_save_path(theme: LevelTheme, size: GridSize, stem: &str) -> PathBuf {
    Path::new(WORKSPACE_ASSETS_ROOT)
        .join(MAPS_SUBDIR)
        .join(theme_dir(theme))
        .join(size_dir(size))
        .join(format!("{stem}.{PREFAB_EXTENSION}"))
}

/// Project the in-memory [`EditorMap`] (incl. multi-level cells) into the canonical GTW-418
/// [`PrefabSpec`] schema — the conversion at the heart of the save (C1 / C2 / C3).
///
/// Iterates every painted `(slot, tile)` of the map, re-checks each through the GTW-430 shared
/// [`evaluate_placement`] predicate (C3 — an illegal cell rejects the whole save), routes each by
/// its editor tile class + catalog kind into the matching prefab list, and derives the C6 edge
/// openings from the walkable boundary cells. The prefab's theme / size / default-floor come from
/// the `session`; the spawn role is the [`SAVED_SPAWN_ROLE`] default.
///
/// # Errors
///
/// [`SavePrefabError::IllegalCell`] if any painted cell is an illegal placement;
/// [`SavePrefabError::NoEdgeOpening`] if no walkable boundary cell exists to derive an opening from.
pub(crate) fn editor_map_to_prefab(
    map: &EditorMap,
    registry: &ThemeCatalogRegistry,
    session: &MapEditorSession,
) -> Result<PrefabSpec, SavePrefabError> {
    let theme = session.theme();
    let size = session.grid_size();
    let mut spec = PrefabSpec {
        theme,
        size,
        spawn_role: SAVED_SPAWN_ROLE,
        default_floor: session
            .default_floor()
            .map_or_else(TerrainName::default, |key| {
                TerrainName::new((**key).clone())
            }),
        ..PrefabSpec::default()
    };

    for (slot, tile) in map.painted() {
        // C3: re-evaluate every painted cell through the shared GTW-430 predicate; an illegal
        // cell rejects the whole save (a saved prefab never contains an illegal cell).
        let placement = ProposedPlacement::new(*slot, tile.clone());
        if let PlacementVerdict::Illegal(_) =
            evaluate_placement(map, registry, theme, &placement, size)
        {
            return Err(SavePrefabError::IllegalCell(*slot));
        }

        let piece = TerrainName::new((**tile).clone());
        match classify(registry, theme, tile) {
            EditorTileClass::Slab => spec.slabs.push(PrefabPiece::new(*slot, piece)),
            EditorTileClass::Ladder => {
                // A ladder rises one storey: author a bidirectional vertical link from this slot
                // to the slot directly above (the only authored way a ganger changes storey).
                if let Some(above) = level_above(*slot) {
                    spec.vertical_links
                        .push(VerticalLink::new(*slot, above, LinkKind::ladder()));
                }
            }
            EditorTileClass::Other => {
                route_other_tile(&mut spec, *slot, piece, registry, theme, tile);
            }
        }
    }

    // C6: derive the edge openings from the walkable boundary cells. A prefab with none cannot
    // connect (the loader would reject it), so reject the save here with a precise reason.
    spec.edge_openings = derive_edge_openings(&spec, size);
    if spec.edge_openings.is_empty() {
        return Err(SavePrefabError::NoEdgeOpening);
    }
    Ok(spec)
}

/// Route an [`Other`](EditorTileClass::Other)-class painted cell into the right prefab list by its
/// catalog kind: a WALL → [`walls`](PrefabSpec::walls), COVER / SCATTER →
/// [`scatter`](PrefabSpec::scatter), FLOOR → a per-cell [`floors`](PrefabSpec::floors) override.
///
/// A tile the registry cannot resolve (a stale key after a theme switch) is treated as a per-cell
/// floor override — the conservative default that preserves the painted cell without inventing a
/// wall / cover.
fn route_other_tile(
    spec: &mut PrefabSpec,
    slot: CellLevel,
    piece: TerrainName,
    registry: &ThemeCatalogRegistry,
    theme: LevelTheme,
    tile: &TileKey,
) {
    let kind = registry
        .catalog(theme)
        .and_then(|catalog| catalog.tile(tile))
        .map(|catalog_tile| catalog_tile.kind);
    match kind {
        Some(CatalogTileKind::Wall(_)) => spec.walls.push(PrefabPiece::new(slot, piece)),
        Some(CatalogTileKind::Cover(_) | CatalogTileKind::Scatter(_)) => {
            spec.scatter.push(PrefabPiece::new(slot, piece));
        }
        // A Floor tile, or an unresolved key, becomes a per-cell floor override. (A Slab kind is
        // routed by the EditorTileClass::Slab arm before this fn is called, so it cannot reach
        // here; it is listed for exhaustiveness only.)
        Some(CatalogTileKind::Floor { .. } | CatalogTileKind::Slab { .. }) | None => {
            spec.floors.push(PrefabPiece::new(slot, piece));
        }
    }
}

/// Derive the C6 [`EdgeOpening`]s for a built [`PrefabSpec`] — every walkable floor cell on a
/// footprint BOUNDARY edge (GTW-432).
///
/// A cell is "walkable" if it carries a per-cell floor override (a [`PrefabPiece`]) OR the prefab
/// has a non-empty default floor (so every open ground-level cell is walkable). A cell is on a
/// boundary edge when its x / y sits on the footprint's perimeter (`0` or `width-1` / `height-1`).
/// The inter-prefab 1-cell seam connects through such cells. Deduplicated + order-stable so the
/// serialized output is deterministic.
#[must_use]
fn derive_edge_openings(spec: &PrefabSpec, size: GridSize) -> Vec<EdgeOpening> {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let has_default_floor = !spec.default_floor.is_empty();

    let mut openings: Vec<EdgeOpening> = Vec::new();
    let push_unique = |slot: CellLevel, openings: &mut Vec<EdgeOpening>| {
        if !openings.iter().any(|o| o.at() == slot) {
            openings.push(EdgeOpening::new(slot));
        }
    };

    // Per-cell floor overrides on the ground-level boundary are explicit walkable seam cells.
    for floor in &spec.floors {
        if floor.at.z == 0 && is_boundary_cell(floor.at, width, height) {
            push_unique(floor.at, &mut openings);
        }
    }

    // With a default floor every open ground-level boundary cell is walkable; expose the
    // perimeter cells NOT occupied by a wall / scatter so the seam has somewhere to connect.
    if has_default_floor {
        for y in 0..height {
            for x in 0..width {
                let slot = CellLevel::new(Cell::new(x, y), Level::new(0));
                if is_boundary_cell(slot, width, height) && !occupies_ground(spec, slot) {
                    push_unique(slot, &mut openings);
                }
            }
        }
    }

    openings
}

/// Whether `slot`'s ground-plane cell sits on the footprint perimeter (x or y on an edge).
#[must_use]
fn is_boundary_cell(slot: CellLevel, width: i32, height: i32) -> bool {
    slot.x == 0 || slot.y == 0 || slot.x == width - 1 || slot.y == height - 1
}

/// Whether a built prefab places a BLOCKING piece (wall / scatter / slab) at `slot` — such a cell
/// is not a walkable seam.
#[must_use]
fn occupies_ground(spec: &PrefabSpec, slot: CellLevel) -> bool {
    spec.walls.iter().any(|c| c.at == slot)
        || spec.scatter.iter().any(|c| c.at == slot)
        || spec.slabs.iter().any(|s| s.at == slot)
}

/// The slot one storey ABOVE `slot`, or [`None`] at the storey ceiling — a ladder's destination.
#[must_use]
fn level_above(slot: CellLevel) -> Option<CellLevel> {
    let next = u8::try_from(slot.z).ok()?.checked_add(1)?;
    if next >= gdtf_battle_sim::metric::MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(Cell::new(slot.x, slot.y), Level::new(next)))
}

/// Serialize a built [`PrefabSpec`] to its `.prefab.ron`-shaped RON text — the SAME schema the
/// GTW-418 loader deserializes (C2). A pretty-printed record so a saved prefab stays
/// human-editable like the shipped `assets/content/maps/**/*.prefab.ron`.
///
/// # Errors
///
/// [`SavePrefabError::Serialize`] wrapping the underlying RON serialization error.
pub(crate) fn serialize_prefab(spec: &PrefabSpec) -> Result<String, SavePrefabError> {
    ron::ser::to_string_pretty(spec, ron::ser::PrettyConfig::default())
        .map_err(|err| SavePrefabError::Serialize(err.to_string()))
}

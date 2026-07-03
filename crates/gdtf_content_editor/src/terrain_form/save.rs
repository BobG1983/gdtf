//! The TERRAIN-mode form's **projection + serialization + write** (GTW-474): turn the in-progress
//! [`TerrainDraft`] into a real [`TerrainDef`], serialize it to a `.terrain_def.ron`, and write it
//! to the active theme's terrain folder so the GTW-487 terrain loader resolves it into the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry).
//!
//! [`draft_to_terrain_def`] + [`serialize_terrain_def`] are PURE (no IO) so a test can round-trip
//! them without touching the assets tree (the C3/C4 round-trip). The filesystem write lives in
//! [`write_terrain_in`] (root-parameterized core, debug-only) and its thin [`write_terrain`] wrapper
//! (the GTW-432 save precedent). Tests call `write_terrain_in` with a unique `tempfile::TempDir` root
//! so they never pollute the version-controlled `assets/` tree.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

#[cfg(debug_assertions)]
use gdtf_assets::FileStem;
use gdtf_assets::serialize_ron_pretty;
use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainUuid},
    piece::TerrainGraphicKey,
};

use super::{draft::TerrainDraft, error::SaveTerrainError, picks::TerrainKindChoice};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_content_editor` → up two levels → `assets`), computed at compile time. So a
/// terrain def the editor SAVES lands exactly where the GTW-487 terrain loader READS from —
/// `assets/content/terrain/<theme>/`.
#[cfg(debug_assertions)]
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The per-theme terrain root the GTW-487 loader scans — `assets/content/terrain/<theme>/`.
/// The SOLE terrain root after GTW-490 (moved under `content/` in GTW-562).
#[cfg(debug_assertions)]
const TERRAIN_SUBDIR: &str = "content/terrain";

/// The compound file extension the GTW-487 terrain loader keys on
/// (`init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"])`) — a saved def MUST
/// use it or the loader never picks the file up.
#[cfg(debug_assertions)]
const TERRAIN_DEF_EXTENSION: &str = "terrain_def.ron";

/// Sanitize the entered display name into a file-name STEM — since GTW-577 a thin
/// delegation to the shared [`gdtf_assets::sanitize_file_stem`] seam (the prefab
/// `sanitize_name` sibling).
///
/// Returns an empty [`FileStem`] for a name that sanitizes to nothing (the caller treats it
/// as [`SaveTerrainError::EmptyName`]).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn sanitize_stem(raw: &str) -> FileStem {
    gdtf_assets::sanitize_file_stem(raw)
}

/// The `snake_case` directory name for a theme, derived from its human label — the prefab
/// `theme_dir` sibling, delegating the slug filter to the shared
/// [`gdtf_assets::sanitize_file_stem`] seam (GTW-577 C1). `assets/content/terrain/<theme>/`
/// keys on the slugified theme DISPLAY NAME (`"Industrial Hive"` → `industrial_hive`),
/// matching the shipped per-theme layout; the empty-slug fallback stays per-type (P9).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn theme_dir(display_name: &str) -> String {
    let slug = gdtf_assets::sanitize_file_stem(display_name);
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug.to_string()
    }
}

/// Project the in-progress [`TerrainDraft`] into a real [`TerrainDef`] keyed by `uuid` (GTW-474
/// C2) — the conversion at the heart of the terrain save.
///
/// Builds the [`sim_kind`](TerrainDef::sim_kind) from the chosen [`TerrainKindChoice`] (the right
/// stat fields per kind — Wall / Cover carry HP + armor + band; Slab carries HP + armor only;
/// Emplacement carries HP + armor + band + the selected mounted-weapon key — GTW-574 C6),
/// the [`presenter_kind`](TerrainDef::presenter_kind) from the chosen graphic role + (Slab-only,
/// C2) footfall, and copies the selected tags. Pure — the caller supplies the (already-minted)
/// `uuid`.
///
/// `pub` (via the crate's [`draft_to_terrain_def`](crate::draft_to_terrain_def) re-export) so the
/// C4 integration test can project the live draft through the SAME conversion the save button runs
/// and assert the produced def round-trips through the GTW-487 loader into a
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry).
///
/// # Errors
///
/// [`SaveTerrainError::MissingMountedWeapon`] for an Emplacement draft with NO selected
/// mounted weapon (GTW-574 C6) — an `Emplacement` sim kind REQUIRES its weapon key, so
/// the projection fails closed: no panic, no silent default, no def produced.
pub fn draft_to_terrain_def(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
) -> Result<TerrainDef, SaveTerrainError> {
    // GTW-566 C5: the draft's graphic is the presenter's TileRole; its `as_key` is the
    // exact authored role string, minted here into the sim's opaque key newtype (the
    // TileRole itself never crosses into the sim).
    let graphic_name = TerrainGraphicKey::new(draft.graphic().as_key().to_owned());
    let (sim_kind, presenter_kind) = match draft.kind() {
        TerrainKindChoice::Wall => (
            TerrainSimKind::Wall {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Wall { graphic_name },
        ),
        TerrainKindChoice::Cover => (
            TerrainSimKind::Cover {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Cover { graphic_name },
        ),
        TerrainKindChoice::Slab => (
            TerrainSimKind::Slab {
                hp:               draft.slab_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
            },
            // C2: footfall is OFFERED only for Slab — the draft forces `None` for any other kind,
            // and `FootfallChoice::footfall` maps `None` → `None`, so a non-slab def never carries
            // a footfall (fail-closed re-validated here by construction).
            TerrainPresenterKind::Slab {
                graphic_name,
                footfall: draft.footfall().footfall(),
            },
        ),
        // GTW-574 C6: an Emplacement is cover-like (CoverHp pool + armor + band) PLUS the
        // mounted-weapon registry key. FAIL-CLOSED: no selected weapon means no def — the
        // sim variant's `mounted_weapon` field is REQUIRED, and defaulting it silently
        // would author a gun out of thin air.
        TerrainKindChoice::Emplacement => {
            let Some(mounted_weapon) = draft.mounted_weapon().cloned() else {
                return Err(SaveTerrainError::MissingMountedWeapon);
            };
            (
                TerrainSimKind::Emplacement {
                    hp: draft.cover_hp(),
                    armor_protection: draft.armor_protection(),
                    armor_hardness: draft.armor_hardness(),
                    height_band: draft.height_band(),
                    mounted_weapon,
                },
                TerrainPresenterKind::Emplacement { graphic_name },
            )
        }
    };
    Ok(TerrainDef {
        key: uuid,
        display_name: TerrainDisplayName::new(draft.display_name().trim().to_owned()),
        sim_kind,
        presenter_kind,
        tags: draft.tags().to_vec(),
        on_death: None,
    })
}

/// Serialize a built [`TerrainDef`] to its `.terrain_def.ron`-shaped RON text — the SAME schema
/// the GTW-487 terrain loader deserializes (C3). Delegates to the shared
/// [`serialize_ron_pretty`] seam (GTW-577 C2), so a saved def stays human-editable like the
/// shipped `assets/content/terrain/**/*.terrain_def.ron`.
///
/// # Errors
///
/// [`SaveTerrainError::Save`] wrapping the seam's serialize failure.
pub fn serialize_terrain_def(def: &TerrainDef) -> Result<String, SaveTerrainError> {
    serialize_ron_pretty(def).map_err(SaveTerrainError::Save)
}

/// Build + serialize + WRITE a terrain def to `<assets_root>/content/terrain/<theme>/<stem>.terrain_def.ron`,
/// or return the typed [`SaveTerrainError`] (never a panic).
///
/// This is the **root-parameterized core** — all path-building, serialization, and `fs` writes go
/// through here. `assets_root` is the on-disk parent of the `content/terrain/` subtree: production passes
/// [`WORKSPACE_ASSETS_ROOT`] (via [`write_terrain`]); tests pass a unique `tempfile::TempDir` root
/// so no test ever writes into the version-controlled `assets/` tree.
///
/// Sanitizes the entered name to a file stem (the shared seam), projects the draft to a
/// [`TerrainDef`] keyed by `uuid`, and hands the serialize → mkdir → write chain to the shared
/// [`gdtf_assets::write_ron_pretty`] seam (GTW-577 C2). Returns the resolved [`PathBuf`] on
/// success so the caller can log it or read it back.
///
/// # Errors
///
/// Any [`SaveTerrainError`] from name validation, the fail-closed Emplacement projection
/// (GTW-574 C6 — a missing mounted weapon writes NOTHING), or the seam's serialization /
/// file write.
#[cfg(debug_assertions)]
pub fn write_terrain_in(
    assets_root: &Path,
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    let stem = sanitize_stem(draft.display_name());
    if stem.is_empty() {
        return Err(SaveTerrainError::EmptyName);
    }
    let def = draft_to_terrain_def(draft, uuid)?;
    let path = assets_root
        .join(TERRAIN_SUBDIR)
        .join(theme_dir(theme_display))
        .join(format!("{stem}.{TERRAIN_DEF_EXTENSION}"));
    gdtf_assets::write_ron_pretty(&path, &def)?;
    Ok(path)
}

/// Build + serialize + WRITE a terrain def to `assets/content/terrain/<theme>/<stem>.terrain_def.ron`
/// (GTW-474 C3), or return the typed [`SaveTerrainError`] (never a panic).
///
/// Thin wrapper around [`write_terrain_in`] that supplies the workspace `assets/` root
/// ([`WORKSPACE_ASSETS_ROOT`]). This is the function the egui Save button calls; the production
/// write path is UNCHANGED — the file lands exactly where the GTW-487 terrain loader reads from.
///
/// # Errors
///
/// Any [`SaveTerrainError`] from name validation, serialization, or the file write.
#[cfg(debug_assertions)]
pub fn write_terrain(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    write_terrain_in(Path::new(WORKSPACE_ASSETS_ROOT), draft, uuid, theme_display)
}

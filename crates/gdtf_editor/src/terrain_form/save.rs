//! Terrain draft conversion and RON save helpers.

#[cfg(feature = "mcp")]
use std::path::{Path, PathBuf};

use cobalt_ron_assets::serialize_ron_pretty;
#[cfg(feature = "mcp")]
use cobalt_ron_assets::{FileStem, workspace_assets_root};
use gdtf_assets::ContentFamily;
use gdtf_battle_sim::terrain::{
    def::{
        BlocksPathingOverride, TerrainDef, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainUuid, TerrainViewArt, TerrainViews, owed_views_for,
    },
    piece::TerrainGraphicKey,
};
#[cfg(feature = "mcp")]
use gdtf_content_families::TerrainDefsFamily;

use super::{draft::TerrainDraft, error::SaveTerrainError, picks::TerrainKindChoice};
#[cfg(feature = "mcp")]
use crate::theme_dir::theme_dir;

#[cfg(feature = "mcp")]
#[must_use]
pub(crate) fn sanitize_stem(raw: &str) -> FileStem {
    cobalt_ron_assets::sanitize_file_stem(raw)
}

// Fills an owed view the draft named no sprite for: its first filled row, else the empty key.
fn fill_key(draft: &TerrainDraft) -> TerrainGraphicKey {
    let owed = owed_views_for(draft.kind().piece_kind(), draft.tags());
    owed.iter()
        .find_map(|view| draft.views().sprite(*view).cloned())
        .unwrap_or_else(|| TerrainGraphicKey::new(String::new()))
}

// Every view the draft owes, filled from the row it holds or from its first filled row.
fn projected_views(draft: &TerrainDraft) -> TerrainViews {
    let fill = fill_key(draft);
    TerrainViews::new(
        owed_views_for(draft.kind().piece_kind(), draft.tags())
            .iter()
            .map(|view| TerrainViewArt {
                view:   *view,
                sprite: draft
                    .views()
                    .sprite(*view)
                    .cloned()
                    .unwrap_or_else(|| fill.clone()),
            })
            .collect(),
    )
}

/// Build a terrain def from a draft and uuid.
///
/// # Errors
///
/// Returns [`SaveTerrainError::MissingMountedWeapon`] when the kind is emplacement and no weapon is set.
pub fn draft_to_terrain_def(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
) -> Result<TerrainDef, SaveTerrainError> {
    let views = projected_views(draft);
    let (sim_kind, presenter_kind) = match draft.kind() {
        TerrainKindChoice::Wall => (
            TerrainSimKind::Wall {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Wall,
        ),
        TerrainKindChoice::Cover => (
            TerrainSimKind::Cover {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Cover,
        ),
        TerrainKindChoice::Slab => (
            TerrainSimKind::Slab {
                hp:               draft.slab_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
            },
            TerrainPresenterKind::Slab {
                footfall: draft.footfall().footfall(),
            },
        ),
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
                    entry_sides: draft.entry_sides().to_vec(),
                },
                TerrainPresenterKind::Emplacement,
            )
        }
    };
    Ok(TerrainDef {
        key: uuid,
        display_name: TerrainDisplayName::new(draft.display_name().trim().to_owned()),
        sim_kind,
        presenter_kind,
        views,
        tags: draft.tags().to_vec(),
        on_death: draft.on_death().to_vec(),
        blocks_pathing: draft.blocks_pathing().map(BlocksPathingOverride::new),
        blocks_los: draft.blocks_los(),
        leaves_behind: draft.leaves_behind().clone(),
    })
}

/// Serialize a terrain def to pretty RON.
///
/// # Errors
///
/// Returns [`SaveTerrainError::Save`] if serialization fails.
pub fn serialize_terrain_def(def: &TerrainDef) -> Result<String, SaveTerrainError> {
    serialize_ron_pretty(def).map_err(SaveTerrainError::Save)
}

/// Write terrain RON under `assets_root`.
///
/// A draft opened from a file writes that file back. A draft that was never opened
/// takes its folder from the session theme and its stem from the display name.
///
/// # Errors
///
/// Returns [`SaveTerrainError`] on empty name, missing weapon, or write failure.
#[cfg(feature = "mcp")]
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
    let path = draft.source().map_or_else(
        || minted_terrain_path(assets_root, &stem, theme_display),
        |source| assets_root.join(&**source),
    );
    cobalt_ron_assets::write_ron_pretty(&path, &def)?;
    Ok(path)
}

// Where a terrain def that was never opened from a file lands.
#[cfg(feature = "mcp")]
fn minted_terrain_path(assets_root: &Path, stem: &FileStem, theme_display: &str) -> PathBuf {
    assets_root
        .join(TerrainDefsFamily::FOLDER)
        .join(theme_dir(theme_display))
        .join(format!("{stem}.{}", TerrainDefsFamily::EXTENSION))
}

/// Write terrain RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`SaveTerrainError`] on empty name, missing weapon, or write failure.
#[cfg(feature = "mcp")]
pub fn write_terrain(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    let Some(root) = workspace_assets_root() else {
        return Err(SaveTerrainError::NoWorkspaceRoot);
    };
    write_terrain_in(&root, draft, uuid, theme_display)
}

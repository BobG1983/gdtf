//! Terrain draft conversion and RON save helpers.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_assets::serialize_ron_pretty;
#[cfg(debug_assertions)]
use gdtf_assets::{ContentFamily, FileStem, workspace_assets_root};
use gdtf_battle_sim::terrain::{
    def::{
        BlocksPathingOverride, TerrainDef, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainUuid, TerrainViewArt, TerrainViews, owed_views_for,
    },
    piece::TerrainGraphicKey,
};
#[cfg(debug_assertions)]
use gdtf_content_families::TerrainDefsFamily;

use super::{draft::TerrainDraft, error::SaveTerrainError, picks::TerrainKindChoice};
#[cfg(debug_assertions)]
use crate::theme_dir::theme_dir;

#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn sanitize_stem(raw: &str) -> FileStem {
    gdtf_assets::sanitize_file_stem(raw)
}

// Every view the draft owes, filled from the row it holds or from the picked graphic.
fn projected_views(draft: &TerrainDraft) -> TerrainViews {
    TerrainViews::new(
        owed_views_for(draft.kind().piece_kind(), draft.tags())
            .iter()
            .map(|view| TerrainViewArt {
                view:   *view,
                sprite: draft.views().sprite(*view).map_or_else(
                    || TerrainGraphicKey::new(draft.graphic().as_key().to_owned()),
                    Clone::clone,
                ),
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
            TerrainPresenterKind::Slab {
                graphic_name,
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
                TerrainPresenterKind::Emplacement { graphic_name },
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
/// # Errors
///
/// Returns [`SaveTerrainError`] on empty name, missing weapon, or write failure.
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
        .join(TerrainDefsFamily::FOLDER)
        .join(theme_dir(theme_display))
        .join(format!("{stem}.{}", TerrainDefsFamily::EXTENSION));
    gdtf_assets::write_ron_pretty(&path, &def)?;
    Ok(path)
}

/// Write terrain RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`SaveTerrainError`] on empty name, missing weapon, or write failure.
#[cfg(debug_assertions)]
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

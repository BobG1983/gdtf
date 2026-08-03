//! (the GTW-432 save precedent). Tests call `write_terrain_in` with a unique `tempfile::TempDir` root
//! so they never pollute the version-controlled `assets/` tree.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_assets::serialize_ron_pretty;
#[cfg(debug_assertions)]
use gdtf_assets::{ContentFamily, FileStem, WORKSPACE_ASSETS_ROOT};
use gdtf_battle_sim::terrain::{
    def::{
        BlocksPathingOverride, TerrainDef, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainUuid,
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

pub fn draft_to_terrain_def(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
) -> Result<TerrainDef, SaveTerrainError> {
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
        blocks_pathing: draft.blocks_pathing().map(BlocksPathingOverride::new),
        blocks_los: draft.blocks_los(),
    })
}

pub fn serialize_terrain_def(def: &TerrainDef) -> Result<String, SaveTerrainError> {
    serialize_ron_pretty(def).map_err(SaveTerrainError::Save)
}

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

#[cfg(debug_assertions)]
pub fn write_terrain(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    write_terrain_in(Path::new(WORKSPACE_ASSETS_ROOT), draft, uuid, theme_display)
}

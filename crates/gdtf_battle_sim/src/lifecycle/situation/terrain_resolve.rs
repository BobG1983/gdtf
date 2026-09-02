//! [`setup_battle`](super::setup_battle) calls to turn each authored terrain DEFINITION
//! migration (child T07a of the data-model refactor): each authored
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    occupancy::PathBlocked,
    situation::BattleSetupError,
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind, TerrainUuid,
            closed_openable_vision_band, derives_path_blocking, derives_vision_occlusion,
            is_openable,
        },
        entity::TerrainPieceKind,
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

pub(crate) struct ResolvedCoverPiece {
    pub(crate) max_hp:           CoverHp,
    pub(crate) height_band:      HeightBand,
    pub(crate) armor_protection: ArmorProtection,
    pub(crate) armor_hardness:   ArmorHardness,
    pub(crate) piece_kind:       TerrainPieceKind,
    pub(crate) graphic:          TerrainGraphicKey,
    pub(crate) blocks_path:      PathBlocked,
    pub(crate) occludes_vision:  Option<HeightBand>,
    pub(crate) openable:         Option<HeightBand>,
    pub(crate) emplacement:      Option<WeaponName>,
    pub(crate) entry_sides:      Vec<TerrainFacing>,
}

pub(crate) struct ResolvedSlabPiece {
    pub(crate) max_hp:           SlabHp,
    pub(crate) armor_protection: ArmorProtection,
    pub(crate) armor_hardness:   ArmorHardness,
    pub(crate) graphic:          TerrainGraphicKey,
    pub(crate) footfall:         Option<FootfallSound>,
    pub(crate) blocks_path:      PathBlocked,
    pub(crate) occludes_vision:  Option<HeightBand>,
    pub(crate) openable:         Option<HeightBand>,
}

pub(crate) fn resolve_cover_def(key: &TerrainUuid, def: &TerrainDef) -> Option<ResolvedCoverPiece> {
    let (max_hp, armor_protection, armor_hardness, height_band, mounted_weapon, entry_sides) =
        match &def.sim_kind {
            TerrainSimKind::Wall {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
            }
            | TerrainSimKind::Cover {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
            } => (
                *hp,
                *armor_protection,
                *armor_hardness,
                *height_band,
                None,
                Vec::new(),
            ),
            TerrainSimKind::Emplacement {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
                mounted_weapon,
                entry_sides,
            } => (
                *hp,
                *armor_protection,
                *armor_hardness,
                *height_band,
                Some(mounted_weapon.clone()),
                entry_sides.clone(),
            ),
            TerrainSimKind::Slab { .. } => {
                bevy::log::error!(
                    "terrain def {:?} is not a Wall/Cover/Emplacement sim-kind (found {:?}); \
                     it cannot be used in walls/scatter — check the situation authoring",
                    key,
                    def.sim_kind,
                );
                return None;
            }
        };
    Some(ResolvedCoverPiece {
        max_hp,
        height_band,
        armor_protection,
        armor_hardness,
        piece_kind: def.sim_kind.kind(),
        emplacement: mounted_weapon,
        entry_sides,
        graphic: presenter_graphic(&def.presenter_kind),
        blocks_path: derives_path_blocking(def),
        occludes_vision: derives_vision_occlusion(def),
        openable: is_openable(def).then(|| closed_openable_vision_band(def)),
    })
}

pub(crate) fn resolve_slab_def(key: &TerrainUuid, def: &TerrainDef) -> Option<ResolvedSlabPiece> {
    let TerrainSimKind::Slab {
        hp,
        armor_protection,
        armor_hardness,
    } = &def.sim_kind
    else {
        bevy::log::error!(
            "terrain def {:?} is not a Slab sim-kind (found {:?}); \
             it cannot be used in slabs — check the situation authoring",
            key,
            def.sim_kind,
        );
        return None;
    };
    Some(ResolvedSlabPiece {
        max_hp:           *hp,
        armor_protection: *armor_protection,
        armor_hardness:   *armor_hardness,
        graphic:          presenter_graphic(&def.presenter_kind),
        footfall:         match &def.presenter_kind {
            TerrainPresenterKind::Slab { footfall, .. } => footfall.clone(),
            TerrainPresenterKind::Wall { .. }
            | TerrainPresenterKind::Cover { .. }
            | TerrainPresenterKind::Emplacement { .. } => None,
        },
        blocks_path:      derives_path_blocking(def),
        occludes_vision:  derives_vision_occlusion(def),
        openable:         is_openable(def).then(|| closed_openable_vision_band(def)),
    })
}

fn presenter_graphic(presenter_kind: &TerrainPresenterKind) -> TerrainGraphicKey {
    match presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name.clone(),
    }
}

pub(super) fn resolve_terrain_or_err<'a>(
    terrain: Option<&'a TerrainDefRegistry>,
    key: &TerrainUuid,
) -> Result<&'a TerrainDef, BattleSetupError> {
    let Some(reg) = terrain else {
        return Err(BattleSetupError::TerrainNotFound { piece: *key });
    };
    reg.def(key)
        .ok_or(BattleSetupError::TerrainNotFound { piece: *key })
}

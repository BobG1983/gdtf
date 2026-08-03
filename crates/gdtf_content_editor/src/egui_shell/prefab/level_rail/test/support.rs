use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

pub(super) const COVER: TerrainUuid = tu(0x01);
pub(super) const SLAB: TerrainUuid = tu(0x02);
pub(super) const UNKNOWN: TerrainUuid = tu(0xFF);

pub(super) fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

fn cover_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Cover".to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn slab_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(50),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

pub(super) fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([(COVER, cover_def(COVER)), (SLAB, slab_def(SLAB))])
}

pub(super) fn texel(image: &egui::ColorImage, x: usize, y: usize) -> egui::Color32 {
    image
        .pixels
        .get(y * image.size[0] + x)
        .copied()
        .unwrap_or(egui::Color32::PLACEHOLDER)
}

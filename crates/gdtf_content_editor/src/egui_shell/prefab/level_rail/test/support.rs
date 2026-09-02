use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    slab::SlabHp,
    terrain::def::{
        LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainTag, TerrainUuid, TerrainViews,
    },
};

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

pub(super) const COVER: TerrainUuid = tu(0x01);
pub(super) const SLAB: TerrainUuid = tu(0x02);
pub(super) const WALL: TerrainUuid = tu(0x03);
pub(super) const DOOR: TerrainUuid = tu(0x04);
pub(super) const STAIR: TerrainUuid = tu(0x05);
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
        presenter_kind: TerrainPresenterKind::Cover,
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
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
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

// A plain wall, carrying no tag at all.
fn wall_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall,
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

// The sim kind and the tag `door_ns.terrain_def.ron` authors.
fn door_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        tags: vec![TerrainTag::Openable],
        display_name: TerrainDisplayName::new("Test Door".to_owned()),
        ..wall_def(key)
    }
}

// The sim kind and the tag `stair_ns_up.terrain_def.ron` authors.
fn stair_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        tags: vec![TerrainTag::Stair],
        display_name: TerrainDisplayName::new("Test Stair".to_owned()),
        ..slab_def(key)
    }
}

pub(super) fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (COVER, cover_def(COVER)),
        (SLAB, slab_def(SLAB)),
        (WALL, wall_def(WALL)),
        (DOOR, door_def(DOOR)),
        (STAIR, stair_def(STAIR)),
    ])
}

pub(super) fn texel(image: &egui::ColorImage, x: usize, y: usize) -> egui::Color32 {
    image
        .pixels
        .get(y * image.size[0] + x)
        .copied()
        .unwrap_or(egui::Color32::PLACEHOLDER)
}

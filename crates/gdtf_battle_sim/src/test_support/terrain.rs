//! Named test terrain definitions and the registry that holds them.

use super::situation::test_pieces;
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag,
        },
        piece::{FootfallSound, TerrainGraphicKey},
    },
};

fn display(name: &str) -> TerrainDisplayName {
    TerrainDisplayName::new(name.to_owned())
}

fn graphic(role: &str) -> TerrainGraphicKey {
    TerrainGraphicKey::new(role.to_owned())
}

fn test_wall() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::WALL,
        display_name:   display("Test Wall"),
        sim_kind:       TerrainSimKind::Wall {
            hp:               CoverHp::new(120),
            armor_protection: ArmorProtection::new(8),
            armor_hardness:   ArmorHardness::new(4),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: graphic("wall"),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_slab() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::SLAB,
        display_name:   display("Test Slab"),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("slab"),
            footfall:     Some(FootfallSound::new("test-step".to_owned())),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_cover() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::COVER,
        display_name:   display("Test Cover"),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(30),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: graphic("cover"),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_floor() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::FLOOR,
        display_name:   display("Test Floor"),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("floor"),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_vision_slab() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::VISION_SLAB,
        display_name:   display("Test Vision Slab"),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("vision-slab"),
            footfall:     None,
        },
        tags:           vec![TerrainTag::BlocksVision],
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_path_slab() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::PATH_SLAB,
        display_name:   display("Test Path Slab"),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("path-slab"),
            footfall:     None,
        },
        tags:           vec![TerrainTag::BlocksPathfinding],
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

fn test_low_vision_cover() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::LOW_VISION_COVER,
        display_name:   display("Test Low Cover"),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(30),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: graphic("low-cover"),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

/// Registry of all named test terrain pieces.
#[must_use]
pub fn test_terrain_registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (test_pieces::WALL, test_wall()),
        (test_pieces::SLAB, test_slab()),
        (test_pieces::COVER, test_cover()),
        (test_pieces::FLOOR, test_floor()),
        (test_pieces::VISION_SLAB, test_vision_slab()),
        (test_pieces::PATH_SLAB, test_path_slab()),
        (test_pieces::LOW_VISION_COVER, test_low_vision_cover()),
    ])
}

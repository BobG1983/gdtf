//! The named test terrain definitions the sim's own fixtures place.

use super::super::{registries::TEST_MOUNTED_WEAPON_KEY, situation::test_pieces};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
            TerrainTag, TerrainView, TerrainViewArt, TerrainViews,
        },
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

pub(super) fn display(name: &str) -> TerrainDisplayName {
    TerrainDisplayName::new(name.to_owned())
}

pub(super) fn graphic(role: &str) -> TerrainGraphicKey {
    TerrainGraphicKey::new(role.to_owned())
}

pub(super) fn view_row(view: TerrainView, sprite: &str) -> TerrainViewArt {
    TerrainViewArt {
        view,
        sprite: graphic(sprite),
    }
}

pub(super) fn test_wall() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_slab() -> TerrainDef {
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
        views:          TerrainViews::new(vec![view_row(TerrainView::Single, "slab")]),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_cover() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_floor() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_vision_slab() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_path_slab() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_low_vision_cover() -> TerrainDef {
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_emplacement() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::EMPLACEMENT,
        display_name:   display("Test Emplacement"),
        sim_kind:       TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            entry_sides:      TerrainFacing::ALL.to_vec(),
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: graphic("emplacement"),
        },
        tags:           Vec::new(),
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

pub(super) fn test_emplacement_leaving_wall() -> TerrainDef {
    TerrainDef {
        key: test_pieces::EMPLACEMENT_LEAVING_WALL,
        display_name: display("Test Emplacement Leaving A Wall"),
        leaves_behind: LeavesBehind::Piece(test_pieces::WALL),
        ..test_emplacement()
    }
}

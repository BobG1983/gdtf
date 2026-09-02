//! Named test terrain definitions and the registry that holds them.

use super::{registries::TEST_MOUNTED_WEAPON_KEY, situation::test_pieces};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainViews,
        },
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

fn test_emplacement() -> TerrainDef {
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

fn test_emplacement_leaving_wall() -> TerrainDef {
    TerrainDef {
        key: test_pieces::EMPLACEMENT_LEAVING_WALL,
        display_name: display("Test Emplacement Leaving A Wall"),
        leaves_behind: LeavesBehind::Piece(test_pieces::WALL),
        ..test_emplacement()
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
        (test_pieces::EMPLACEMENT, test_emplacement()),
        (
            test_pieces::EMPLACEMENT_LEAVING_WALL,
            test_emplacement_leaving_wall(),
        ),
    ])
}

#[cfg(test)]
mod test {
    use super::{TerrainSimKind, test_terrain_registry};
    use crate::{test_support::registries::test_weapon_registry, weapon::WeaponName};

    #[test]
    fn every_test_emplacement_names_a_weapon_the_test_registry_holds() {
        let registry = test_terrain_registry();
        let mounts: Vec<&WeaponName> = registry
            .defs()
            .filter_map(|(_, def)| match &def.sim_kind {
                TerrainSimKind::Emplacement { mounted_weapon, .. } => Some(mounted_weapon),
                TerrainSimKind::Wall { .. }
                | TerrainSimKind::Cover { .. }
                | TerrainSimKind::Slab { .. } => None,
            })
            .collect();
        assert!(
            !mounts.is_empty(),
            "test_terrain_registry must hold at least one Emplacement def, or the mounted-weapon \
             check below passes without checking anything",
        );

        let weapons = test_weapon_registry();
        for key in mounts {
            assert!(
                weapons.spec(key).is_some(),
                "test_weapon_registry must resolve `{}`, the mounted weapon a test emplacement \
                 def names — the two registries are handed out as a pair",
                key.as_str(),
            );
        }
    }
}

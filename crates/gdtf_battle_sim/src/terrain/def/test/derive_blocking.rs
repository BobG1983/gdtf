use super::super::{
    TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
    derives_path_blocking, sim_kind_blocks_path,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::TerrainGraphicKey,
    weapon::WeaponName,
};

fn wall_def(tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn cover_def(tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Cover".to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Mid,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("crate".to_owned()),
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn emplacement_def(tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Emplacement".to_owned()),
        sim_kind: TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new("heavy_bolter".to_owned()),
            entry_sides:      Vec::new(),
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn slab_def(tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

#[test]
fn wall_blocks_path_by_kind_default() {
    assert!(
        *derives_path_blocking(&wall_def(vec![])),
        "a Wall blocks the path by kind default (zero regression)",
    );
}

#[test]
fn cover_blocks_path_by_kind_default() {
    assert!(
        *derives_path_blocking(&cover_def(vec![])),
        "a Cover blocks the path by kind default (zero regression)",
    );
}

#[test]
fn emplacement_blocks_path_by_kind_default() {
    assert!(
        *derives_path_blocking(&emplacement_def(vec![])),
        "an Emplacement blocks the path by kind default (a cover-like structure)",
    );
}

#[test]
fn slab_does_not_block_path_by_default() {
    assert!(
        !*derives_path_blocking(&slab_def(vec![])),
        "a Slab does NOT block the path by default",
    );
}

#[test]
fn explicit_tag_adds_path_blocking_to_slab() {
    assert!(
        *derives_path_blocking(&slab_def(vec![TerrainTag::BlocksPathfinding])),
        "an explicit BlocksPathfinding tag makes an otherwise-open Slab block the path",
    );
}

#[test]
fn unrelated_tag_does_not_block_slab() {
    assert!(
        !*derives_path_blocking(&slab_def(vec![
            TerrainTag::BlocksVision,
            TerrainTag::Openable
        ])),
        "a Slab with only BlocksVision/Openable tags does NOT block the path",
    );
}

#[test]
fn explicit_tag_on_wall_is_still_blocking() {
    assert!(
        *derives_path_blocking(&wall_def(vec![TerrainTag::BlocksPathfinding])),
        "a tagged Wall is still path-blocking (the union is monotone)",
    );
}

#[test]
fn sim_kind_default_blocks_wall_and_cover_only() {
    let wall = TerrainSimKind::Wall {
        hp:               CoverHp::new(1),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::High,
    };
    let cover = TerrainSimKind::Cover {
        hp:               CoverHp::new(1),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::Low,
    };
    let slab = TerrainSimKind::Slab {
        hp:               SlabHp::new(1),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
    };
    let emplacement = TerrainSimKind::Emplacement {
        hp:               CoverHp::new(1),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::High,
        mounted_weapon:   WeaponName::new("heavy_bolter".to_owned()),
        entry_sides:      Vec::new(),
    };
    assert!(*sim_kind_blocks_path(&wall), "Wall blocks by default");
    assert!(*sim_kind_blocks_path(&cover), "Cover blocks by default");
    assert!(
        *sim_kind_blocks_path(&emplacement),
        "Emplacement blocks by default (a cover-like structure)"
    );
    assert!(
        !*sim_kind_blocks_path(&slab),
        "Slab does not block by default"
    );
}

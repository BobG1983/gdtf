use super::super::{
    LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainTag,
    TerrainUuid, derives_vision_occlusion, sim_kind_occludes_vision,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::TerrainGraphicKey,
    weapon::WeaponName,
};

fn wall_def(band: HeightBand, tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      band,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn cover_def(band: HeightBand, tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Cover".to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      band,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("crate".to_owned()),
        },
        tags,
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn emplacement_def(band: HeightBand, tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Emplacement".to_owned()),
        sim_kind: TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      band,
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
        leaves_behind: LeavesBehind::Nothing,
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
        leaves_behind: LeavesBehind::Nothing,
    }
}

#[test]
fn wall_occludes_fully_by_kind_default() {
    for band in [HeightBand::High, HeightBand::Mid, HeightBand::Low] {
        assert_eq!(
            derives_vision_occlusion(&wall_def(band, vec![])),
            Some(HeightBand::High),
            "an untagged Wall occludes vision FULLY (whole storey → High) by kind default, \
             regardless of its authored band ({band:?})",
        );
    }
}

#[test]
fn cover_occludes_at_its_band_by_default() {
    assert_eq!(
        derives_vision_occlusion(&cover_def(HeightBand::Mid, vec![])),
        Some(HeightBand::Mid),
        "a Cover occludes vision at its authored band (Mid here — read from the def)",
    );
    assert_eq!(
        derives_vision_occlusion(&cover_def(HeightBand::Low, vec![])),
        Some(HeightBand::Low),
        "a Low-band Cover occludes at Low — the band is the def's, not a constant",
    );
}

#[test]
fn emplacement_occludes_at_its_band_by_default() {
    assert_eq!(
        derives_vision_occlusion(&emplacement_def(HeightBand::High, vec![])),
        Some(HeightBand::High),
        "an Emplacement occludes vision at its authored band (High here — read from the def)",
    );
    assert_eq!(
        derives_vision_occlusion(&emplacement_def(HeightBand::Mid, vec![])),
        Some(HeightBand::Mid),
        "a Mid-band Emplacement occludes at Mid — the band is the def's, not a constant",
    );
}

#[test]
fn slab_does_not_occlude_by_default() {
    assert_eq!(
        derives_vision_occlusion(&slab_def(vec![])),
        None,
        "a Slab does NOT occlude vision by default",
    );
}

#[test]
fn explicit_tag_adds_high_occlusion_to_slab() {
    assert_eq!(
        derives_vision_occlusion(&slab_def(vec![TerrainTag::BlocksVision])),
        Some(HeightBand::High),
        "an explicit BlocksVision tag makes an otherwise-transparent Slab occlude at HIGH",
    );
}

#[test]
fn unrelated_tag_does_not_occlude_slab() {
    assert_eq!(
        derives_vision_occlusion(&slab_def(vec![
            TerrainTag::BlocksPathfinding,
            TerrainTag::Openable
        ])),
        None,
        "a Slab with only BlocksPathfinding/Openable tags does NOT occlude vision",
    );
}

#[test]
fn explicit_tag_on_wall_keeps_its_band() {
    assert_eq!(
        derives_vision_occlusion(&wall_def(HeightBand::Mid, vec![TerrainTag::BlocksVision])),
        Some(HeightBand::Mid),
        "a tagged Wall occludes at its OWN band (Mid here), not a forced HIGH",
    );
}

#[test]
fn sim_kind_default_occludes_wall_and_cover_only() {
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
    assert!(*sim_kind_occludes_vision(&wall), "Wall occludes by default");
    assert!(
        *sim_kind_occludes_vision(&cover),
        "Cover occludes by default"
    );
    assert!(
        !*sim_kind_occludes_vision(&slab),
        "Slab does not occlude by default"
    );
}

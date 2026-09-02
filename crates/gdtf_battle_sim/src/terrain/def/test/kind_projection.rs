use super::super::{TerrainPresenterKind, TerrainSimKind};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{entity::TerrainPieceKind, piece::FootfallSound},
    weapon::WeaponName,
};

fn sim_kind_of(kind: TerrainPieceKind) -> TerrainSimKind {
    match kind {
        TerrainPieceKind::Wall => TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        TerrainPieceKind::Cover => TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        TerrainPieceKind::Slab => TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
        },
        TerrainPieceKind::Emplacement => TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new("heavy_bolter".to_owned()),
            entry_sides:      Vec::new(),
        },
    }
}

fn presenter_kind_of(kind: TerrainPieceKind) -> TerrainPresenterKind {
    match kind {
        TerrainPieceKind::Wall => TerrainPresenterKind::Wall,
        TerrainPieceKind::Cover => TerrainPresenterKind::Cover,
        TerrainPieceKind::Slab => TerrainPresenterKind::Slab {
            footfall: Some(FootfallSound::new("footfall_metal".to_owned())),
        },
        TerrainPieceKind::Emplacement => TerrainPresenterKind::Emplacement,
    }
}

#[test]
fn every_sim_kind_projects_to_its_piece_kind() {
    for kind in TerrainPieceKind::ALL {
        assert_eq!(
            sim_kind_of(kind).kind(),
            kind,
            "TerrainSimKind::kind() must project the {kind:?} sim variant back onto \
             TerrainPieceKind::{kind:?} ",
        );
    }
}

#[test]
fn every_presenter_kind_projects_to_its_piece_kind() {
    for kind in TerrainPieceKind::ALL {
        assert_eq!(
            presenter_kind_of(kind).kind(),
            kind,
            "TerrainPresenterKind::kind() must project the {kind:?} presenter variant back \
             onto TerrainPieceKind::{kind:?} ",
        );
    }
}

#[test]
fn piece_kind_inventory_is_complete_and_distinct() {
    const fn ordinal(kind: TerrainPieceKind) -> usize {
        match kind {
            TerrainPieceKind::Wall => 0,
            TerrainPieceKind::Cover => 1,
            TerrainPieceKind::Slab => 2,
            TerrainPieceKind::Emplacement => 3,
        }
    }
    let mut seen = [false; TerrainPieceKind::ALL.len()];
    for kind in TerrainPieceKind::ALL {
        let slot = ordinal(kind);
        assert!(
            !seen[slot],
            "TerrainPieceKind::ALL must not repeat a variant ({kind:?})",
        );
        seen[slot] = true;
    }
    assert!(
        seen.iter().all(|covered| *covered),
        "TerrainPieceKind::ALL must cover every variant exactly once ",
    );
}

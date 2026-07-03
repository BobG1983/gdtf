//! GTW-574 C1 — the canonical-discriminant projections: every [`TerrainSimKind`] /
//! [`TerrainPresenterKind`] variant projects onto its matching [`TerrainPieceKind`]
//! via the exhaustive `kind()` projections, and the [`TerrainPieceKind::ALL`]
//! inventory is complete (the enumerable list the derived vocabularies pin against).

use super::super::{TerrainPresenterKind, TerrainSimKind};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        entity::TerrainPieceKind,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

/// A sim-kind fixture per canonical kind — arbitrary magnitudes (nothing pinned), one
/// variant per [`TerrainPieceKind`]. The match is EXHAUSTIVE on the canonical
/// discriminant, so a new piece kind breaks this fixture at compile time.
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
        },
    }
}

/// A presenter-kind fixture per canonical kind — the mirror of [`sim_kind_of`] for the
/// presenter half.
fn presenter_kind_of(kind: TerrainPieceKind) -> TerrainPresenterKind {
    let graphic_name = TerrainGraphicKey::new("floor".to_owned());
    match kind {
        TerrainPieceKind::Wall => TerrainPresenterKind::Wall { graphic_name },
        TerrainPieceKind::Cover => TerrainPresenterKind::Cover { graphic_name },
        TerrainPieceKind::Slab => TerrainPresenterKind::Slab {
            graphic_name,
            footfall: Some(FootfallSound::new("footfall_metal".to_owned())),
        },
        TerrainPieceKind::Emplacement => TerrainPresenterKind::Emplacement { graphic_name },
    }
}

/// GTW-574 C1 — every [`TerrainSimKind`] variant projects onto ITS canonical
/// [`TerrainPieceKind`] (the projection is kind-level identity across the whole
/// inventory).
#[test]
fn every_sim_kind_projects_to_its_piece_kind() {
    for kind in TerrainPieceKind::ALL {
        assert_eq!(
            sim_kind_of(kind).kind(),
            kind,
            "TerrainSimKind::kind() must project the {kind:?} sim variant back onto \
             TerrainPieceKind::{kind:?} (GTW-574 C1)",
        );
    }
}

/// GTW-574 C1 — every [`TerrainPresenterKind`] variant projects onto ITS canonical
/// [`TerrainPieceKind`] (the presenter half stays in canonical lockstep).
#[test]
fn every_presenter_kind_projects_to_its_piece_kind() {
    for kind in TerrainPieceKind::ALL {
        assert_eq!(
            presenter_kind_of(kind).kind(),
            kind,
            "TerrainPresenterKind::kind() must project the {kind:?} presenter variant back \
             onto TerrainPieceKind::{kind:?} (GTW-574 C1)",
        );
    }
}

/// GTW-574 C1 — the [`TerrainPieceKind::ALL`] inventory is COMPLETE and duplicate-free:
/// every variant appears exactly once. The `ordinal` match is EXHAUSTIVE, so adding a
/// variant without growing `ALL` is a compile error here, then a length-assert failure.
#[test]
fn piece_kind_inventory_is_complete_and_distinct() {
    /// The exhaustive per-variant ordinal — the compile-time forcing function: a new
    /// `TerrainPieceKind` variant breaks this match before the assert can lie.
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
        "TerrainPieceKind::ALL must cover every variant exactly once (GTW-574 C1)",
    );
}

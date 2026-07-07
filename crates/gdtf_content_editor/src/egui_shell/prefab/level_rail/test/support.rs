//! Shared fixtures for the level-rail model tests — the tiny terrain registry (a
//! `cover`-role def + a `slab`-role def), the test grid, and the texel probe.

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

/// A terrain UUID built from a small constant (the test registry's keys).
const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

/// The `cover`-role def key.
pub(super) const COVER: TerrainUuid = tu(0x01);
/// The `slab`-role def key.
pub(super) const SLAB: TerrainUuid = tu(0x02);
/// A key NO registry holds (the fallback-hue probe).
pub(super) const UNKNOWN: TerrainUuid = tu(0xFF);

/// A `4 × 4 × 3` drawable volume, with a `1 × 1 × 1` fallback (fallible ctor; the
/// fallback keeps the test panic-free per the workspace lints).
pub(super) fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

/// A COVER terrain def whose graphic role is `cover` (magnitudes are throwaway data).
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
    }
}

/// A SLAB terrain def whose graphic role is `slab` (magnitudes are throwaway data).
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
    }
}

/// A registry resolving [`COVER`] to the `cover` role and [`SLAB`] to the `slab` role.
pub(super) fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([(COVER, cover_def(COVER)), (SLAB, slab_def(SLAB))])
}

/// The texel at `(x, y)` of a swept image (row-major, top-left origin).
pub(super) fn texel(image: &egui::ColorImage, x: usize, y: usize) -> egui::Color32 {
    image
        .pixels
        .get(y * image.size[0] + x)
        .copied()
        .unwrap_or(egui::Color32::PLACEHOLDER)
}

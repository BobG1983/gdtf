//! GTW-502 (C1) — the vision-occlusion DERIVATION rule
//! ([`derives_vision_occlusion`](super::super::derives_vision_occlusion)): a `Wall`/`Cover`
//! sim-kind occludes vision at its OWN [`HeightBand`] by default (zero regression — the SAME
//! band its `CoverLedger` entry already occludes at), a `Slab` does NOT, and an explicit
//! [`BlocksVision`](super::super::TerrainTag) tag ADDS occlusion (banded HIGH for a tagged
//! Slab, which has no band of its own).

use super::super::{
    TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
    derives_vision_occlusion, sim_kind_occludes_vision,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::TerrainGraphicKey,
    weapon::WeaponName,
};

/// A `Wall` def at `band` with the given tags — the high-cover blocking kind.
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
    }
}

/// A `Cover` def at `band` with the given tags — the chest-high / scatter kind.
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
    }
}

/// An `Emplacement` def at `band` with the given tags — the cover-like mounted-gun kind
/// (GTW-543; occludes vision at its own band by default, like a `Wall`/`Cover`).
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
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags,
    }
}

/// A `Slab` def with the given tags — the floor/roof kind (NOT vision-occluding by default).
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
    }
}

/// C1 — a `Wall` with NO tags occludes vision at its OWN band (the kind default; existing
/// walls keep occluding sight at the SAME band their `CoverLedger` entry does — zero
/// regression).
#[test]
fn wall_occludes_at_its_band_by_default() {
    assert_eq!(
        derives_vision_occlusion(&wall_def(HeightBand::High, vec![])),
        Some(HeightBand::High),
        "a Wall occludes vision at its authored band by kind default (zero regression)",
    );
}

/// C1 — a `Cover` with NO tags occludes vision at its OWN band (the kind default), and the
/// band is read from the def, not hardcoded (a Mid-band cover occludes at Mid).
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

/// GTW-543 — an `Emplacement` with NO tags occludes vision at its OWN band (the kind default,
/// like a `Wall`/`Cover`), and the band is read from the def, not hardcoded.
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

/// C1 — a `Slab` with NO tags does NOT occlude vision (a slab is a z-boundary the slab march
/// already stops sight at, not a same-storey occluder).
#[test]
fn slab_does_not_occlude_by_default() {
    assert_eq!(
        derives_vision_occlusion(&slab_def(vec![])),
        None,
        "a Slab does NOT occlude vision by default",
    );
}

/// C1 — an EXPLICIT `BlocksVision` tag ADDS occlusion to an otherwise-transparent `Slab`,
/// banded HIGH (a slab has no band of its own; an opaque slab fills the storey).
#[test]
fn explicit_tag_adds_high_occlusion_to_slab() {
    assert_eq!(
        derives_vision_occlusion(&slab_def(vec![TerrainTag::BlocksVision])),
        Some(HeightBand::High),
        "an explicit BlocksVision tag makes an otherwise-transparent Slab occlude at HIGH",
    );
}

/// C1 — an unrelated tag does NOT make a `Slab` occlude vision: only the `BlocksVision` tag
/// (or a Wall/Cover kind default) does. Pins that the rule reads the SPECIFIC tag.
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

/// C1 — an explicit `BlocksVision` tag on a `Wall` keeps it occluding at its OWN band (the
/// tag bands a `Wall`/`Cover` at the `sim_kind` band, NOT a forced HIGH — so a Mid wall tagged
/// `BlocksVision` still occludes at Mid). The union is monotone (a tag never removes
/// occlusion).
#[test]
fn explicit_tag_on_wall_keeps_its_band() {
    assert_eq!(
        derives_vision_occlusion(&wall_def(HeightBand::Mid, vec![TerrainTag::BlocksVision])),
        Some(HeightBand::Mid),
        "a tagged Wall occludes at its OWN band (Mid here), not a forced HIGH",
    );
}

/// C1 — the per-kind default predicate directly: `Wall`/`Cover` occlude, `Slab` does not (the
/// `sim_kind_blocks_path` mirror).
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
    assert!(sim_kind_occludes_vision(&wall), "Wall occludes by default");
    assert!(
        sim_kind_occludes_vision(&cover),
        "Cover occludes by default"
    );
    assert!(
        !sim_kind_occludes_vision(&slab),
        "Slab does not occlude by default"
    );
}

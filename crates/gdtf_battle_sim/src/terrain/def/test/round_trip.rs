//! C4 — round-trip IDENTITY: `deserialize(serialize(def)) == def` for one Wall, one
//! Cover, and one Slab definition. NO magnitude pins — identity only.

use super::super::{
    TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::{FootfallSound, TerrainGraphicKey},
    weapon::WeaponName,
};

/// Serialize `def` to RON and parse it back, asserting the round-trip is the
/// identity. Returns silently (no panic) if either serde step fails, after asserting
/// it succeeded — the no-`unwrap`/`expect` house style.
fn assert_round_trips(def: &TerrainDef) {
    let serialized = ron::ser::to_string(def);
    assert!(
        serialized.is_ok(),
        "a TerrainDef must serialize to RON: {serialized:?}",
    );
    let Ok(text) = serialized else { return };

    let reparsed = ron::de::from_str::<TerrainDef>(&text);
    assert!(
        reparsed.is_ok(),
        "the serialized RON must parse back into a TerrainDef: {reparsed:?} (from {text})",
    );
    let Ok(round_tripped) = reparsed else { return };

    assert_eq!(
        &round_tripped, def,
        "deserialize(serialize(def)) must equal def (round-trip identity)",
    );
}

/// C4 — a Wall definition round-trips to itself (identity). Carries a couple of
/// sim-owned tags to exercise the tag vec through the round-trip too.
#[test]
fn wall_def_round_trips() {
    let def = TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0001)),
        display_name:   TerrainDisplayName::new("Bulkhead Wall".to_owned()),
        sim_kind:       TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags:           vec![TerrainTag::BlocksVision, TerrainTag::BlocksPathfinding],
        on_death:       None,
    };
    assert_round_trips(&def);
}

/// C4 — a Cover definition round-trips to itself (identity). Empty tags exercise the
/// default-empty path through the round-trip.
#[test]
fn cover_def_round_trips() {
    let def = TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0002)),
        display_name:   TerrainDisplayName::new("Supply Crate".to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    assert_round_trips(&def);
}

/// C4 — a Slab definition round-trips to itself (identity), including the optional
/// footfall on the presenter side.
#[test]
fn slab_def_round_trips() {
    let def = TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0003)),
        display_name:   TerrainDisplayName::new("Deck Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     Some(FootfallSound::new("footfall_metal".to_owned())),
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    assert_round_trips(&def);
}

/// GTW-543 — an `Emplacement` definition round-trips to itself (identity), including the
/// mounted-weapon key on the sim side.
#[test]
fn emplacement_def_round_trips() {
    let def = TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0004)),
        display_name:   TerrainDisplayName::new("Heavy Bolter Emplacement".to_owned()),
        sim_kind:       TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new("heavy_bolter".to_owned()),
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    assert_round_trips(&def);
}

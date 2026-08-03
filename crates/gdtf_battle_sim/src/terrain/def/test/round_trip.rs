use super::super::{
    BlocksPathingOverride, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
    TerrainTag, TerrainUuid,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::{FootfallSound, TerrainGraphicKey},
    weapon::WeaponName,
};

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

        blocks_pathing: None,
        blocks_los:     None,
    };
    assert_round_trips(&def);
}

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

        blocks_pathing: None,
        blocks_los:     None,
    };
    assert_round_trips(&def);
}

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

        blocks_pathing: None,
        blocks_los:     None,
    };
    assert_round_trips(&def);
}

fn slab_with_path_override(over: Option<BlocksPathingOverride>) -> TerrainDef {
    TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0587)),
        display_name:   TerrainDisplayName::new("Override Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(80),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       None,

        blocks_pathing: over,
        blocks_los:     None,
    }
}

/// GTW-705 (special item) — the [`BlocksPathingOverride`] wrap is `#[serde(transparent)]`, so an
#[test]
fn blocks_pathing_override_serializes_transparently() {
    for over in [
        Some(BlocksPathingOverride::new(true)),
        Some(BlocksPathingOverride::new(false)),
        None,
    ] {
        let def = slab_with_path_override(over);
        assert_round_trips(&def);

        let serialized = ron::ser::to_string(&def);
        assert!(serialized.is_ok(), "def must serialize: {serialized:?}");
        let Ok(text) = serialized else { return };
        assert!(
            !text.contains("BlocksPathingOverride"),
            "the newtype is #[serde(transparent)] — its name must NOT appear in the RON wire form \
             (the override rides as a bare bool, identical to the pre-wrap Option<bool>): \
             {text}",
        );
        match over {
            Some(o) if *o => assert!(
                text.contains("blocks_pathing:Some(true)"),
                "an authored Some(true) rides as the bare-bool wire form: {text}",
            ),
            Some(_) => assert!(
                text.contains("blocks_pathing:Some(false)"),
                "an authored Some(false) rides as the bare-bool wire form: {text}",
            ),
            None => assert!(
                text.contains("blocks_pathing:None"),
                "an omitted override rides as None: {text}",
            ),
        }
    }
}

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

        blocks_pathing: None,
        blocks_los:     None,
    };
    assert_round_trips(&def);
}

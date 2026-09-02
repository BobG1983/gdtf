use super::super::{
    BlocksPathingOverride, LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind,
    TerrainSimKind, TerrainTag, TerrainUuid, TerrainViewArt, TerrainViews, owed_views_for,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        entity::TerrainPieceKind,
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

// Every view this kind and tag set owes, each row naming the one sprite key.
fn owed_art(kind: TerrainPieceKind, tags: &[TerrainTag], sprite: &str) -> TerrainViews {
    TerrainViews::new(
        owed_views_for(kind, tags)
            .iter()
            .map(|view| TerrainViewArt {
                view:   *view,
                sprite: TerrainGraphicKey::new(sprite.to_owned()),
            })
            .collect(),
    )
}

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
        views:          owed_art(
            TerrainPieceKind::Wall,
            &[TerrainTag::BlocksVision, TerrainTag::BlocksPathfinding],
            "wall",
        ),
        tags:           vec![TerrainTag::BlocksVision, TerrainTag::BlocksPathfinding],
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          owed_art(TerrainPieceKind::Cover, &[], "cover"),
        tags:           Vec::new(),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          owed_art(TerrainPieceKind::Slab, &[], "slab"),
        tags:           Vec::new(),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
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
        views:          owed_art(TerrainPieceKind::Slab, &[], "floor"),
        tags:           Vec::new(),
        on_death:       Vec::new(),

        blocks_pathing: over,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

/// (special item) — the [`BlocksPathingOverride`] wrap is `#[serde(transparent)]`, so an
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
            entry_sides:      vec![TerrainFacing::South, TerrainFacing::West],
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        views:          owed_art(TerrainPieceKind::Emplacement, &[], "cover"),
        tags:           Vec::new(),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    };
    assert_round_trips(&def);
}

#[test]
fn a_def_with_no_leaves_behind_key_parses_as_nothing() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000005",
        display_name: "Plain Wall",
        sim_kind: Wall(
            hp: 40,
            armor_protection: 6,
            armor_hardness: 3,
            height_band: High,
        ),
        presenter_kind: Wall(
            graphic_name: "wall",
        ),
        views: [
            (view: Edge(North), sprite: "wall"),
            (view: Edge(East), sprite: "wall"),
            (view: Edge(South), sprite: "wall"),
            (view: Edge(West), sprite: "wall"),
            (view: Corner(NorthEast), sprite: "wall"),
            (view: Corner(SouthEast), sprite: "wall"),
            (view: Corner(SouthWest), sprite: "wall"),
            (view: Corner(NorthWest), sprite: "wall"),
        ],
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "a def that omits leaves_behind must still parse, which is what #[serde(default)] buys: \
         {parsed:?}",
    );
    let Ok(def) = parsed else { return };
    assert_eq!(
        def.leaves_behind,
        LeavesBehind::Nothing,
        "an omitted leaves_behind reads as Nothing, so no shipped content has to migrate",
    );
}

#[test]
fn a_def_that_leaves_a_piece_behind_round_trips() {
    let successor = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0006));
    let def = TerrainDef {
        key:            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0007)),
        display_name:   TerrainDisplayName::new("Collapsing Wall".to_owned()),
        sim_kind:       TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        views:          owed_art(TerrainPieceKind::Wall, &[], "wall"),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Piece(successor),
    };
    assert_round_trips(&def);

    let serialized = ron::ser::to_string(&def);
    assert!(serialized.is_ok(), "def must serialize: {serialized:?}");
    let Ok(text) = serialized else { return };
    let reparsed = ron::de::from_str::<TerrainDef>(&text);
    let Ok(round_tripped) = reparsed else {
        unreachable!("assert_round_trips already parsed this text")
    };
    assert_eq!(
        round_tripped.leaves_behind,
        LeavesBehind::Piece(successor),
        "the successor key comes back as the same key, not as another def and not as Nothing",
    );
}

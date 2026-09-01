use super::super::{TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag};
use crate::{effects::on_death::OnDeathEffect, terrain::piece::TerrainGraphicKey};

#[test]
fn wall_named_struct_ron_parses() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000001",
        display_name: "Bulkhead Wall",
        sim_kind: Wall(
            hp: 40,
            armor_protection: 6,
            armor_hardness: 3,
            height_band: High,
        ),
        presenter_kind: Wall(
            graphic_name: "wall",
        ),
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(parsed.is_ok(), "Wall TerrainDef must parse: {parsed:?}");
    if let Ok(def) = parsed {
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Wall { .. }),
            "sim_kind must be the Wall struct variant, got {:?}",
            def.sim_kind,
        );
        assert!(
            matches!(def.presenter_kind, TerrainPresenterKind::Wall { .. }),
            "presenter_kind must be the Wall struct variant, got {:?}",
            def.presenter_kind,
        );
    }
}

#[test]
fn cover_named_struct_ron_parses() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000002",
        display_name: "Supply Crate",
        sim_kind: Cover(
            hp: 20,
            armor_protection: 3,
            armor_hardness: 1,
            height_band: Low,
        ),
        presenter_kind: Cover(
            graphic_name: "cover",
        ),
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(parsed.is_ok(), "Cover TerrainDef must parse: {parsed:?}");
    if let Ok(def) = parsed {
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Cover { .. }),
            "sim_kind must be the Cover struct variant, got {:?}",
            def.sim_kind,
        );
        assert!(
            matches!(def.presenter_kind, TerrainPresenterKind::Cover { .. }),
            "presenter_kind must be the Cover struct variant, got {:?}",
            def.presenter_kind,
        );
    }
}

#[test]
fn slab_named_struct_ron_parses() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000003",
        display_name: "Deck Slab",
        sim_kind: Slab(
            hp: 120,
            armor_protection: 5,
            armor_hardness: 2,
        ),
        presenter_kind: Slab(
            graphic_name: "slab",
            footfall: Some("footfall_metal"),
        ),
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(parsed.is_ok(), "Slab TerrainDef must parse: {parsed:?}");
    if let Ok(def) = parsed {
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Slab { .. }),
            "sim_kind must be the Slab struct variant, got {:?}",
            def.sim_kind,
        );
        assert!(
            matches!(def.presenter_kind, TerrainPresenterKind::Slab { .. }),
            "presenter_kind must be the Slab struct variant, got {:?}",
            def.presenter_kind,
        );
    }
}

#[test]
fn emplacement_omitting_entry_sides_parses_with_none() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000004",
        display_name: "Scenery Mount",
        sim_kind: Emplacement(
            hp: 45,
            armor_protection: 5,
            armor_hardness: 2,
            height_band: High,
            mounted_weapon: "heavy_bolter",
        ),
        presenter_kind: Emplacement(
            graphic_name: "emplacement",
        ),
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "an Emplacement TerrainDef that omits entry_sides must parse: {parsed:?}",
    );
    if let Ok(def) = parsed {
        assert!(
            matches!(
                &def.sim_kind,
                TerrainSimKind::Emplacement { entry_sides, .. } if entry_sides.is_empty()
            ),
            "an omitted entry_sides reads as no sides — a mount authored as scenery, got {:?}",
            def.sim_kind,
        );
    }
}

#[test]
fn slab_presenter_footfall_is_optional() {
    let with_footfall = r#"(
        key: "01840a3e-0000-4000-8000-000000000010",
        display_name: "Deck Slab",
        sim_kind: Slab(hp: 120, armor_protection: 5, armor_hardness: 2),
        presenter_kind: Slab(graphic_name: "slab", footfall: Some("footfall_metal")),
    )"#;
    let with = ron::de::from_str::<TerrainDef>(with_footfall);
    assert!(with.is_ok(), "Slab with footfall must parse: {with:?}");
    if let Ok(def) = with {
        assert!(
            matches!(
                def.presenter_kind,
                TerrainPresenterKind::Slab {
                    footfall: Some(_),
                    ..
                }
            ),
            "Slab presenter_kind must carry the Some(footfall), got {:?}",
            def.presenter_kind,
        );
    }

    let without_footfall = r#"(
        key: "01840a3e-0000-4000-8000-000000000011",
        display_name: "Deck Slab",
        sim_kind: Slab(hp: 120, armor_protection: 5, armor_hardness: 2),
        presenter_kind: Slab(graphic_name: "slab", footfall: None),
    )"#;
    let without = ron::de::from_str::<TerrainDef>(without_footfall);
    assert!(
        without.is_ok(),
        "Slab with footfall: None must parse: {without:?}"
    );
    if let Ok(def) = without {
        assert!(
            matches!(
                def.presenter_kind,
                TerrainPresenterKind::Slab { footfall: None, .. }
            ),
            "Slab presenter_kind must carry footfall None, got {:?}",
            def.presenter_kind,
        );
    }

    let _wall: TerrainPresenterKind = TerrainPresenterKind::Wall {
        graphic_name: TerrainGraphicKey::new("wall".to_owned()),
    };
    let _cover: TerrainPresenterKind = TerrainPresenterKind::Cover {
        graphic_name: TerrainGraphicKey::new("cover".to_owned()),
    };
}

#[test]
fn a_def_authoring_two_effects_parses_them_in_order() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000030",
        display_name: "Fuel Barrel",
        sim_kind: Cover(
            hp: 20,
            armor_protection: 3,
            armor_hardness: 1,
            height_band: Low,
        ),
        presenter_kind: Cover(
            graphic_name: "cover",
        ),
        on_death: [
            Explode(hit_type: Blast(radius: 1), damage: 8, damage_type: Blast),
            LeaveField(field: "burning"),
        ],
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "a def authoring two on-death effects must parse: {parsed:?}",
    );
    if let Ok(def) = parsed {
        assert!(
            matches!(
                def.on_death.as_slice(),
                [
                    OnDeathEffect::Explode { .. },
                    OnDeathEffect::LeaveField { .. }
                ]
            ),
            "the parsed list is the two authored effects, Explode then LeaveField, got {:?}",
            def.on_death,
        );
    }
}

#[test]
fn a_def_omitting_on_death_parses_as_an_empty_list() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000031",
        display_name: "Supply Crate",
        sim_kind: Cover(
            hp: 20,
            armor_protection: 3,
            armor_hardness: 1,
            height_band: Low,
        ),
        presenter_kind: Cover(
            graphic_name: "cover",
        ),
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "a def omitting on_death must parse: {parsed:?}",
    );
    if let Ok(def) = parsed {
        assert!(
            def.on_death.is_empty(),
            "an omitted on_death reads as an empty list, got {:?}",
            def.on_death,
        );
    }
}

#[test]
fn sim_kind_and_tag_inventory_is_closed() {
    fn assert_sim_kind_inventory(kind: &TerrainSimKind) {
        match kind {
            TerrainSimKind::Wall { .. }
            | TerrainSimKind::Cover { .. }
            | TerrainSimKind::Slab { .. }
            | TerrainSimKind::Emplacement { .. } => {}
        }
    }

    fn assert_tag_inventory(tag: TerrainTag) {
        match tag {
            TerrainTag::Openable
            | TerrainTag::BlocksVision
            | TerrainTag::BlocksPathfinding
            | TerrainTag::Indestructible => {}
        }
    }

    let slab = ron::de::from_str::<TerrainDef>(
        r#"(
            key: "01840a3e-0000-4000-8000-000000000020",
            display_name: "Deck Slab",
            sim_kind: Slab(hp: 120, armor_protection: 5, armor_hardness: 2),
            presenter_kind: Slab(graphic_name: "slab", footfall: None),
        )"#,
    );
    assert!(slab.is_ok(), "inventory fixture must parse: {slab:?}");
    if let Ok(def) = slab {
        assert_sim_kind_inventory(&def.sim_kind);
    }
    for tag in [
        TerrainTag::Openable,
        TerrainTag::BlocksVision,
        TerrainTag::BlocksPathfinding,
        TerrainTag::Indestructible,
    ] {
        assert_tag_inventory(tag);
    }
}

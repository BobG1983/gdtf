//! C1 / C3 / C5 — named-struct RON parse per `sim_kind`, the `presenter_kind`
//! footfall shape, and the variant inventory.
//!
//! No magnitude assertions — the spot numbers are fixtures (mechanism, not balance);
//! these tests assert STRUCTURE / variant shape only (the brittle-test rule).

use super::super::{TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag};
use crate::terrain::piece::TerrainGraphicKey;

/// C1 — a NAMED-STRUCT RON literal for the `Wall` `sim_kind` parses into a
/// [`TerrainDef`] via [`ron::de::from_str`]. The `Wall(hp: ..., ...)` single-paren
/// named-struct shape proves the variant is a struct variant (NOT the double-paren
/// tuple shape `Wall((...))`).
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

/// C1 — a NAMED-STRUCT RON literal for the `Cover` `sim_kind` parses into a
/// [`TerrainDef`]. Scatter folds into `Cover` (there is no `Scatter` variant).
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

/// C1 — a NAMED-STRUCT RON literal for the `Slab` `sim_kind` parses into a
/// [`TerrainDef`]. A slab carries NO height band (`resolution.md` §2).
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

/// C3 — a `Slab` `presenter_kind` accepts an OPTIONAL footfall: it parses both WITH a
/// `footfall: Some(_)` and WITHOUT (the field can be `None`), while `Wall` / `Cover`
/// `presenter_kind`s carry NO footfall field at all.
#[test]
fn slab_presenter_footfall_is_optional() {
    // Slab WITH a footfall.
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

    // Slab WITHOUT a footfall (the Option is None).
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

    // Structural proof, IN CODE, that Wall / Cover presenter_kinds carry ONLY a
    // graphic_name (no footfall). Constructing each variant by NAMING every field —
    // and the compiler accepting exactly `{ graphic_name }` with no other field —
    // is the type-level guarantee that these variants have no footfall. (A grep of
    // `kind.rs` confirms the same: footfall appears only on the Slab variant.)
    let _wall: TerrainPresenterKind = TerrainPresenterKind::Wall {
        graphic_name: TerrainGraphicKey::new("wall".to_owned()),
    };
    let _cover: TerrainPresenterKind = TerrainPresenterKind::Cover {
        graphic_name: TerrainGraphicKey::new("cover".to_owned()),
    };
}

/// C5 — variant inventory. An EXHAUSTIVE match over [`TerrainSimKind`] compiles with
/// exactly the `Wall` / `Cover` / `Slab` arms — proving there is NO `Floor` and NO
/// `Scatter` variant (a missing/extra arm would not compile). [`TerrainTag`] is the
/// closed 4-variant enum, asserted exhaustively the same way.
#[test]
fn sim_kind_and_tag_inventory_is_closed() {
    // Exhaustive over TerrainSimKind: Wall / Cover / Slab ONLY (no Floor, no Scatter).
    fn assert_sim_kind_inventory(kind: &TerrainSimKind) {
        match kind {
            TerrainSimKind::Wall { .. }
            | TerrainSimKind::Cover { .. }
            | TerrainSimKind::Slab { .. } => {}
        }
    }

    // Exhaustive over TerrainTag: exactly Openable / BlocksVision / BlocksPathfinding
    // / Indestructible.
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

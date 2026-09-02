use super::super::{
    LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
    TerrainSimKind, TerrainUuid, TerrainViews,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    slab::SlabHp,
    terrain::piece::{FootfallSound, TerrainGraphicKey},
};

#[test]
fn registry_inserts_and_looks_up_by_uuid() {
    let key = TerrainUuid::generate();
    let def = TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Deck Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     Some(FootfallSound::new("footfall_metal".to_owned())),
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    };

    let mut registry = TerrainDefRegistry::default();
    assert!(registry.is_empty(), "a fresh registry is empty");

    let previous = registry.insert(key, def.clone());
    assert!(
        previous.is_none(),
        "the first insert under a key has no predecessor"
    );
    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted definition"
    );
    assert!(
        !registry.is_empty(),
        "a one-definition registry is non-empty"
    );

    assert_eq!(
        registry.def(&key),
        Some(&def),
        "a present key resolves to its definition",
    );
    assert!(
        registry.def(&TerrainUuid::generate()).is_none(),
        "a fresh, never-inserted key resolves to None",
    );
}

#[test]
fn registry_new_keys_by_uuid() {
    let key = TerrainUuid::generate();
    let def = TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Deck Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(5),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    };

    let registry = TerrainDefRegistry::new([(key, def.clone())]);
    assert_eq!(
        registry.len(),
        1,
        "the constructor holds the one definition"
    );
    assert_eq!(
        registry.def(&key),
        Some(&def),
        "the constructed registry resolves the inserted key",
    );
}

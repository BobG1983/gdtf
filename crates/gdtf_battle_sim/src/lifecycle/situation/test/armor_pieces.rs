use bevy::ecs::relationship::{Relationship, RelationshipTarget};

use super::support::*;
use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, Wears, WornBy,
};

fn single_ganger_setup() -> Option<(bevy::app::App, crate::situation::BattleSetup)> {
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0)])
        .build();
    run_setup(situation)
}

fn expected_piece() -> crate::armor::ArmorPiece {
    crate::test_support::test_armor_spec().pieces()[BodyPart::Head.index()]
}

#[test]
fn setup_relates_six_body_part_tagged_pieces_to_the_ganger() {
    let Some((app, setup)) = single_ganger_setup() else {
        return;
    };
    assert!(
        !setup.occupants.is_empty(),
        "the single-ganger setup must spawn one occupant",
    );
    let Some(placement) = setup.occupants.first() else {
        return;
    };
    let ganger = placement.occupant;

    let wears = app.world().get::<Wears>(ganger);
    assert!(
        wears.is_some(),
        "the spawned ganger must carry a `Wears` relationship-target collection",
    );
    let Some(wears) = wears else { return };
    let pieces: Vec<_> = wears.iter().collect();
    assert_eq!(
        pieces.len(),
        6,
        "the ganger must wear exactly six armor-piece entities (one per BodyPart)",
    );

    let mut tags: Vec<BodyPart> = Vec::new();
    for &piece in &pieces {
        let worn_by = app.world().get::<WornBy>(piece);
        assert!(
            worn_by.is_some_and(|w| w.get() == ganger),
            "each related piece must carry `WornBy(ganger)` pointing at the wearer",
        );
        if let Some(part) = app.world().get::<BodyPart>(piece) {
            tags.push(*part);
        }
    }
    for part in BodyPart::ALL {
        assert!(
            tags.contains(&part),
            "the worn pieces must tag every BodyPart — {part:?} is missing",
        );
    }
}

#[test]
fn each_worn_piece_carries_its_resolved_stat_components() {
    let Some((app, setup)) = single_ganger_setup() else {
        return;
    };
    assert!(
        !setup.occupants.is_empty(),
        "the single-ganger setup must spawn one occupant",
    );
    let Some(placement) = setup.occupants.first() else {
        return;
    };
    let ganger = placement.occupant;
    let wears = app.world().get::<Wears>(ganger);
    assert!(
        wears.is_some(),
        "the spawned ganger must carry a `Wears` collection",
    );
    let Some(wears) = wears else { return };

    let want = expected_piece();
    for piece in wears.iter() {
        let floor = app.world().get::<ArmorFloor>(piece).map(|c| **c);
        let protection = app.world().get::<ArmorProtection>(piece).map(|c| **c);
        let integrity = app.world().get::<ArmorIntegrity>(piece).map(|c| **c);
        let hardness = app.world().get::<ArmorHardness>(piece).map(|c| **c);
        let armor_type = app.world().get::<ArmorType>(piece).copied();
        assert_eq!(
            floor,
            Some(*want.floor),
            "the piece entity must carry the resolved ArmorFloor component",
        );
        assert_eq!(
            protection,
            Some(*want.protection),
            "the piece entity must carry the resolved ArmorProtection component",
        );
        assert_eq!(
            integrity,
            Some(*want.integrity),
            "the piece entity must carry the resolved ArmorIntegrity component",
        );
        assert_eq!(
            hardness,
            Some(*want.hardness),
            "the piece entity must carry the resolved ArmorHardness component",
        );
        assert_eq!(
            armor_type,
            Some(want.armor_type),
            "the piece entity must carry the resolved ArmorType component",
        );
    }
}

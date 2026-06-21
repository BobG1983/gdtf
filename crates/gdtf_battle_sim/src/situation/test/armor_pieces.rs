//! GTW-323 slice 1 (ADR-0004) — `setup_battle` spawns each ganger's armor as related
//! **piece entities** via `Wears`, one per [`BodyPart`], each carrying its stat
//! components read from the registry-resolved [`ArmorSpec`].
//!
//! These tests drive the REAL `setup_battle` → `queue_spawn_related_scenes::<Wears>`
//! path (through [`run_setup`], which `app.update()`s once to materialize the deferred
//! scene spawns), then query the world to prove the relationship + per-piece stats.

use bevy::ecs::relationship::RelationshipTarget;

use super::support::*;
use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, Wears, WornBy,
};

/// A single-ganger fixture (one authored ganger at an arbitrary cell) so a test reads
/// exactly one ganger's worn pieces — the spawned entity is the first occupant.
fn single_ganger_setup() -> Option<(bevy::app::App, crate::situation::BattleSetup)> {
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0)])
        .build();
    run_setup(situation)
}

/// The default test armor suit (`test_armor_spec` = `arbitrary_armor(1)`): the uniform
/// per-piece stats every spawned piece must carry — read straight off the resolved
/// spec so the assertion never pins a hand-copied magnitude.
fn expected_piece() -> crate::armor::ArmorPiece {
    crate::test_support::test_armor_spec().pieces()[BodyPart::Head.index()]
}

/// GTW-323 — `setup_battle` relates SIX worn-armor-piece entities to the spawned
/// ganger via `Wears`, one for each [`BodyPart`]. Proves the `ganger → Wears → pieces`
/// edge is built (the relationship machinery + the six-piece spawn), and that every
/// piece is tagged `With<WornBy>` (the back-reference points at the ganger).
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

    // The ganger carries a `Wears` collection of exactly six piece entities.
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

    // Each related piece is a worn piece (`WornBy` back-reference present) and the six
    // BodyPart tags are exactly the six canonical parts (no dupes, none missing).
    let mut tags: Vec<BodyPart> = Vec::new();
    for &piece in &pieces {
        let worn_by = app.world().get::<WornBy>(piece);
        assert!(
            worn_by.is_some_and(|w| w.0 == ganger),
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

/// GTW-323 — each spawned piece entity carries the FIVE per-piece stat components
/// (`ArmorFloor` / `ArmorProtection` / `ArmorIntegrity` / `ArmorHardness` / `ArmorType`)
/// read **by value** from the registry-resolved [`ArmorSpec`] — the same values the
/// transient `WornArmor` blob copies, but now living ON the piece entity. Proves the
/// `struck_piece` read path has real per-piece stats to resolve against (not array
/// slots).
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
        // Read every stat component off the piece ENTITY (not a WornArmor slot).
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

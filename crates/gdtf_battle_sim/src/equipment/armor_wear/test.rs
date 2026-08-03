//! from the former inline `#[cfg(test)] mod tests`).

use bevy::prelude::{App, Entity, MessageReader, MessageWriter, MinimalPlugins, Update, World};

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    armor_wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome, wear_armor},
    resolve_hit::IntegrityWear,
};

fn piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

fn a_ganger() -> Entity {
    World::new().spawn_empty().id()
}

#[test]
fn wear_past_zero_leaves_piece_unprotected() {
    let mut integrity = piece_integrity(3);
    let ganger = a_ganger();

    let _broke = wear_armor(
        &mut integrity,
        BodyPart::Torso,
        IntegrityWear::new(10),
        ganger,
    );

    assert!(
        *integrity <= 0,
        "wear past the starting value must leave integrity ≤ 0 (bare flesh thereafter)",
    );
}

#[test]
fn emits_exactly_once_on_the_crossing() {
    let mut integrity = piece_integrity(5);
    let ganger = a_ganger();

    let first = wear_armor(
        &mut integrity,
        BodyPart::LeftArm,
        IntegrityWear::new(6),
        ganger,
    );
    assert_eq!(
        first,
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, BodyPart::LeftArm)),
        "the protecting→broken crossing must emit Broke(ArmorBroken) once",
    );

    let second = wear_armor(
        &mut integrity,
        BodyPart::LeftArm,
        IntegrityWear::new(6),
        ganger,
    );
    assert_eq!(
        second,
        ArmorWearOutcome::Unaffected,
        "an already-broken piece that re-wears must NOT re-emit (emit once)",
    );
}

#[test]
fn worn_not_broken_when_still_protecting() {
    let mut integrity = piece_integrity(10);
    let ganger = a_ganger();

    let result = wear_armor(
        &mut integrity,
        BodyPart::RightLeg,
        IntegrityWear::new(4),
        ganger,
    );

    assert_eq!(
        result,
        ArmorWearOutcome::Damaged(ArmorDamaged::new(
            ganger,
            BodyPart::RightLeg,
            IntegrityWear::new(4)
        )),
        "a hit that leaves integrity > 0 must surface Damaged(delta=4), never Broke",
    );
    assert!(
        *integrity > 0,
        "the piece must still protect (integrity > 0) after a sub-fatal wear",
    );
}

#[test]
fn zero_wear_on_protecting_piece_is_unaffected() {
    let mut integrity = piece_integrity(10);
    let ganger = a_ganger();

    let result = wear_armor(
        &mut integrity,
        BodyPart::RightLeg,
        IntegrityWear::new(0),
        ganger,
    );

    assert_eq!(
        result,
        ArmorWearOutcome::Unaffected,
        "a zero-wear hit must surface nothing (no misleading Armor -0)",
    );
    assert!(
        *integrity > 0,
        "a zero-wear hit must leave the piece protecting (integrity > 0)",
    );
}

/// `#[derive(Message)]`, NOT the observer `Event` API. Builds a headless app
#[test]
fn armor_broken_is_a_buffered_message_read_in_a_headless_app() {
    use bevy::prelude::{IntoScheduleConfigs, Resource};

            #[derive(Resource, Default)]
    struct Captured(Vec<ArmorBroken>);

    let ganger = a_ganger();
    let part = BodyPart::Head;

    let produce = move |mut writer: MessageWriter<ArmorBroken>| {
        let mut integrity = piece_integrity(1);
        if let ArmorWearOutcome::Broke(broke) =
            wear_armor(&mut integrity, part, IntegrityWear::new(2), ganger)
        {
            writer.write(broke);
        }
    };

    let consume = |mut reader: MessageReader<ArmorBroken>,
                   mut captured: bevy::prelude::ResMut<Captured>| {
        for broke in reader.read() {
            captured.0.push(*broke);
        }
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<ArmorBroken>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (produce, consume).chain());

    app.update();

    let captured = app
        .world()
        .get_resource::<Captured>()
        .map_or_else(Vec::new, |c| c.0.clone());

    assert_eq!(
        captured.len(),
        1,
        "exactly one ArmorBroken message must be read from the buffer",
    );
    assert_eq!(
        captured.first(),
        Some(&ArmorBroken::new(ganger, part)),
        "the buffered ArmorBroken must carry the right ganger Entity + BodyPart",
    );
}

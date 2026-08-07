use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::ThrowGrenadeRequested,
    ganger::{Direction, Hp, Wounds},
    surface::{SlabState, SurfaceGrid},
    test_support::SituationBuilder,
};

use super::harness::*;

fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (
        app.world().get::<Hp>(entity).map(|h| **h),
        app.world().get::<Wounds>(entity).map(|w| **w),
    )
}

const fn took_damage(before: (Option<u16>, Option<u8>), after: (Option<u16>, Option<u8>)) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

fn set_roof(app: &mut App, cells: &[(i32, i32)], state: SlabState) {
    let mut surface = app.world_mut().resource_mut::<SurfaceGrid>();
    for &(x, y) in cells {
        surface.set_slab(at_level(x, y, 1), state);
    }
}

#[test]
fn a_grenade_lobbed_through_a_roof_hole_damages_the_room_occupants() {
    let (mut app, seed) = battle_app(0x5546_0A0A, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Destroyed,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    assert!(
        took_damage(a0, vitals(&app, occ_a)),
        "the direct room occupant (5,7) is damaged by the lobbed blast (before {a0:?}, after {:?})",
        vitals(&app, occ_a),
    );
    assert!(
        took_damage(b0, vitals(&app, occ_b)),
        "the adjacent room occupant (6,7) is caught by the radius-1 blast (before {b0:?}, after {:?})",
        vitals(&app, occ_b),
    );
}

#[test]
fn a_grenade_lobbed_at_an_intact_roof_is_blocked_and_spares_the_room() {
    let (mut app, seed) = battle_app(0x5546_0B0B, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Present,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    assert_eq!(
        vitals(&app, occ_a),
        a0,
        "an intact roof spares the room occupant (5,7) — the lob is blocked above it",
    );
    assert_eq!(
        vitals(&app, occ_b),
        b0,
        "an intact roof spares the room occupant (6,7)",
    );
}

#[test]
fn a_blind_throw_resolves_without_a_facing_or_los_gate() {
    let (mut app, seed) = battle_app(0x5546_0C0C, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(ground(5, 5), Direction::North),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(thrower_e), Some(victim)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the thrower + the target");
    };
    let v0 = vitals(&app, victim);

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(9, 5)));
    step(&mut app, 3);

    assert!(
        took_damage(v0, vitals(&app, victim)),
        "a blind lob (thrower facing away) still resolves and damages the target — no facing / \
         LOS gate for an Arc weapon (before {v0:?}, after {:?})",
        vitals(&app, victim),
    );
}

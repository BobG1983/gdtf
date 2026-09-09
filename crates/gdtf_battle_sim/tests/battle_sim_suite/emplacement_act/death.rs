//! A killed occupant: the body stays on the mount cell, the seat frees, the next ganger mounts.
//! A downed occupant keeps the seat, its occupant record and the gun, and admits nobody else.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::EnterEmplacementRequested,
    effects::{
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            ImmuneArmorTypes,
        },
        on_death::{OnDeathEffect, OnDeathOccurred},
    },
    ganger::{Direction, LifeState},
    metric::CellLevel,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, TEST_MOUNTED_WEAPON_KEY, emplacement_at, field_turns},
    weapon::{DamageType, WeaponName, WeaponRegistry, WeaponSpec},
};

use super::harness::*;

/// The seed both cases drive, so a rerun differs only in what the case writes.
const SEED: u64 = 0x5543_1235;

/// Ticks a request is given to reach the toggle and settle on the grid.
const SETTLE_TICKS: u32 = 3;

/// Move a ganger's life state the way a hit does: write the state asked for and nothing else.
fn set_life(app: &mut App, ganger: Entity, life_state: LifeState) {
    let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) else {
        unreachable!("every seeded ganger carries a LifeState");
    };
    *life = life_state;
}

/// Two players flanking a seat: the first mans it, is killed, and the death is settled.
/// Yields the emplacement, the corpse on it, and the ganger still standing on the far side.
fn a_seat_with_a_dead_gunner() -> (App, Entity, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(west_entry(), Direction::East),
            player_at(east_entry(), Direction::West),
        ])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let first = player_on(&mut app, west_entry());
    let second = player_on(&mut app, east_entry());
    mount(&mut app, first, emplacement);
    set_life(&mut app, first, LifeState::Dead);
    step(&mut app, SETTLE_TICKS);
    (app, emplacement, first, second)
}

/// Two players flanking a seat: the first mans it, is downed, and the down is settled.
/// Yields the emplacement, the downed gunner on it, and the ganger still standing on the far side.
fn a_seat_with_a_downed_gunner() -> (App, Entity, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(west_entry(), Direction::East),
            player_at(east_entry(), Direction::West),
        ])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let first = player_on(&mut app, west_entry());
    let second = player_on(&mut app, east_entry());
    mount(&mut app, first, emplacement);
    set_life(&mut app, first, LifeState::Downed);
    step(&mut app, SETTLE_TICKS);
    (app, emplacement, first, second)
}

#[test]
fn a_death_frees_the_seat_and_the_next_ganger_takes_the_gun() {
    let (mut app, emplacement, first, second) = a_seat_with_a_dead_gunner();

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "a killed occupant gives the seat up; the state reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "a killed occupant's record is dropped; the seat names {:?}",
        occupant(&app, emplacement),
    );
    assert_eq!(
        mount_entity(&app, emplacement),
        None,
        "a killed occupant's mounted weapon is despawned; the seat holds {:?}",
        mount_entity(&app, emplacement),
    );
    assert_eq!(
        pos_of(&app, first),
        Some(seat()),
        "the corpse stays on the mount cell {:?}; it lies on {:?}",
        seat(),
        pos_of(&app, first),
    );
    assert_eq!(
        grid_occupant(&app, seat()),
        None,
        "the death releases the seat's grid slot and nothing re-claims it; the grid names {:?}",
        grid_occupant(&app, seat()),
    );

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(second, emplacement));
    step(&mut app, SETTLE_TICKS);

    assert_eq!(
        occupant(&app, emplacement),
        Some(second),
        "the freed seat takes a new occupant; it names {:?}",
        occupant(&app, emplacement),
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "the new occupant gets a mounted weapon of its own; the seat holds {:?}",
        mount_entity(&app, emplacement),
    );
    assert!(
        wields_mount(&mut app, second),
        "the new occupant wields the mount rather than sitting in an empty seat; it wields {}",
        wields_mount(&mut app, second),
    );
    assert_eq!(
        grid_occupant(&app, seat()),
        Some(second),
        "the grid names the new occupant on the seat; it names {:?}",
        grid_occupant(&app, seat()),
    );
}

#[test]
fn the_freed_seat_admits_a_second_ganger_with_a_working_gun() {
    let (mut app, emplacement, _corpse, second) = a_seat_with_a_dead_gunner();

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(second, emplacement));
    step(&mut app, SETTLE_TICKS);

    assert_eq!(
        occupant(&app, emplacement),
        Some(second),
        "a seat freed by a death admits a ganger standing on one of its entry sides; it names \
         {:?}",
        occupant(&app, emplacement),
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "the seat spawns a mounted weapon for the new occupant; it holds {:?}",
        mount_entity(&app, emplacement),
    );
    assert!(
        wields_mount(&mut app, second),
        "the new occupant wields that mount rather than sitting in an empty seat; it wields {}",
        wields_mount(&mut app, second),
    );
}

#[test]
fn a_downed_gunner_keeps_the_seat_the_record_and_the_gun() {
    let (app, emplacement, first, _second) = a_seat_with_a_downed_gunner();

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "a downed occupant holds the seat; the state reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "a downed occupant keeps its record on the seat; the seat names {:?}",
        occupant(&app, emplacement),
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "a downed occupant keeps the mounted weapon; the seat holds {:?}",
        mount_entity(&app, emplacement),
    );
}

#[test]
fn a_downed_gunner_shuts_the_seat_to_the_next_ganger() {
    let (mut app, emplacement, first, second) = a_seat_with_a_downed_gunner();
    let tu_before = tu_of(&app, second);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(second, emplacement));
    step(&mut app, SETTLE_TICKS);

    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the seat a downed gunner holds takes no new occupant; it names {:?}",
        occupant(&app, emplacement),
    );
    assert_eq!(
        tu_of(&app, second),
        tu_before,
        "the refused enter spends nothing, so the pool stays at {tu_before:?}; it reads {:?}",
        tu_of(&app, second),
    );
}

/// The catalog key the MOUNTED weapon's authored death effect names.
fn burning() -> FieldKey {
    FieldKey::new("burning".to_owned())
}

/// The catalog key the CARRIED gun's authored death effect names.
fn own_field() -> FieldKey {
    FieldKey::new("scorched".to_owned())
}

/// A registry where both guns author a death field, keyed apart so the fan names which fired.
fn mounted_on_death_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            WeaponSpec {
                on_death: vec![OnDeathEffect::LeaveField { field: own_field() }],
                ..gun_spec(OWN_DAMAGE_TYPE)
            },
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            WeaponSpec {
                on_death: vec![OnDeathEffect::LeaveField { field: burning() }],
                ..gun_spec(MOUNT_DAMAGE_TYPE)
            },
        ),
    ])
}

/// The field catalog both effects resolve through; battle setup inserts none of its own.
/// The two defs differ in damage type, which is what a placed field remembers about its def.
fn burning_catalog() -> FieldDefRegistry {
    FieldDefRegistry::new([
        (burning(), field_def(MOUNT_DAMAGE_TYPE)),
        (own_field(), field_def(OWN_DAMAGE_TYPE)),
    ])
}

/// A field def told apart from its sibling by the channel it burns on.
fn field_def(damage_type: DamageType) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(3),
        damage_type,
        ImmuneArmorTypes::default(),
        FieldDuration::Turns(field_turns(2)),
    )
}

/// Whether a field stands on a cell.
fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|fields| fields.field_at(&at).is_some())
}

/// The channel the field standing on a cell burns on, which names the def it was placed from.
fn field_damage_type(app: &App, at: CellLevel) -> Option<DamageType> {
    app.world()
        .get_resource::<FieldRegistry>()
        .and_then(|fields| fields.field_at(&at))
        .map(|placed| placed.def().damage_type)
}

#[test]
fn a_gunner_killed_at_the_mount_fires_the_gun_on_their_back() {
    let (mut app, seed) = battle_app(SEED);
    app.insert_resource(mounted_on_death_registry());
    let situation = SituationBuilder::new()
        .with_gangers([player_at(west_entry(), Direction::East)])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    app.insert_resource(burning_catalog());
    let emplacement = seated_emplacement(&mut app, seat());
    let actor = player_on(&mut app, west_entry());
    mount(&mut app, actor, emplacement);
    assert!(
        wields_mount(&mut app, actor),
        "PRECONDITION: the actor must wield the mount, or a carried-gun field below proves \
         nothing about which gun the death read; it wields {}",
        wields_mount(&mut app, actor),
    );
    assert!(
        !field_present(&app, seat()),
        "PRECONDITION: no field stands on the seat before the death; one already does",
    );

    set_life(&mut app, actor, LifeState::Dead);
    app.world_mut()
        .write_message(OnDeathOccurred::new(actor, seat()));
    step(&mut app, SETTLE_TICKS);

    assert_eq!(
        field_damage_type(&app, seat()),
        Some(OWN_DAMAGE_TYPE),
        "a gunner's death fans the gun on their back, so the field on {:?} is the carried gun's \
         {OWN_DAMAGE_TYPE:?} one and not the mount's {MOUNT_DAMAGE_TYPE:?}; it burns {:?}",
        seat(),
        field_damage_type(&app, seat()),
    );
}

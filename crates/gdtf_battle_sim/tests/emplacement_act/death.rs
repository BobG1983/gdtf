//! A killed occupant: the body stays on the mount cell, the seat frees, the next ganger mounts.

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
    ganger::{Direction, LifeState, Position},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    prelude::Faction,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, TEST_MOUNTED_WEAPON_KEY, emplacement_at, field_turns},
    weapon::{DamageType, WeaponName, WeaponRegistry, WeaponSpec},
};

use super::harness::*;

/// The seed both cases drive, so a rerun differs only in what the case writes.
const SEED: u64 = 0x5543_1235;

/// Ticks a request is given to reach the toggle and settle on the grid.
const SETTLE_TICKS: u32 = 3;

/// The cell the emplacement is seeded on.
fn seat() -> CellLevel {
    ground(6, 5)
}

/// The entry side the first occupant starts on.
fn west_entry() -> CellLevel {
    ground(5, 5)
}

/// The entry side the second ganger starts on.
fn east_entry() -> CellLevel {
    ground(7, 5)
}

/// The one player ganger standing on `at`, or a failure naming the cell and the count found.
fn player_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, faction, position)| ***faction == PLAYER && ***position == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one player ganger must stand on {at:?}, found {}",
        found.len(),
    );
    let [entity] = found[..] else {
        unreachable!("the count above is one");
    };
    entity
}

/// Kill a ganger the way a fatal hit does: write `LifeState::Dead` and nothing else.
fn kill(app: &mut App, ganger: Entity) {
    let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) else {
        unreachable!("every seeded ganger carries a LifeState");
    };
    *life = LifeState::Dead;
}

/// Who the OCCUPANCY GRID says stands on a cell, which `occupant` does not answer.
fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|grid| grid.occupant(&at))
}

/// Seat the first ganger and fail unless the enter actually manned the emplacement.
fn mount(app: &mut App, actor: Entity, emplacement: Entity) {
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(app, SETTLE_TICKS);
    assert_eq!(
        state(app, emplacement),
        Some(EmplacementState::Occupied),
        "PRECONDITION: the enter must man the seat, or the clear below has nothing to clear and \
         every assertion passes for nothing; it reads {:?}",
        state(app, emplacement),
    );
    assert_eq!(
        occupant(app, emplacement),
        Some(actor),
        "PRECONDITION: the seat must name this actor as its occupant — a failed enter leaves \
         MountedBy absent and the clear has nothing to find; it names {:?}",
        occupant(app, emplacement),
    );
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
    kill(&mut app, first);
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

/// The catalog key the mounted weapon's authored death effect names.
fn burning() -> FieldKey {
    FieldKey::new("burning".to_owned())
}

/// A registry whose MOUNTED spec alone authors an on-death effect; the carried gun authors none.
fn mounted_on_death_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            WeaponSpec {
                on_death: Some(OnDeathEffect::LeaveField { field: burning() }),
                ..gun_spec(MOUNT_DAMAGE_TYPE)
            },
        ),
    ])
}

/// The field catalog the effect resolves through; battle setup inserts none of its own.
fn burning_catalog() -> FieldDefRegistry {
    FieldDefRegistry::new([(
        burning(),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    )])
}

/// Whether a field stands on a cell.
fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|fields| fields.field_at(&at).is_some())
}

#[test]
fn a_gunner_killed_at_the_mount_fires_the_mounts_on_death_effect() {
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
        "PRECONDITION: the actor must wield the mount, or the effect read below is the carried \
         gun's; it wields {}",
        wields_mount(&mut app, actor),
    );
    assert!(
        !field_present(&app, seat()),
        "PRECONDITION: no field stands on the seat before the death; one already does",
    );

    kill(&mut app, actor);
    app.world_mut()
        .write_message(OnDeathOccurred::new(actor, seat()));
    step(&mut app, SETTLE_TICKS);

    assert!(
        field_present(&app, seat()),
        "the death resolves against the mount that was there when the gunner died, so the \
         mount's LeaveField lands on {:?}; the registry holds {}",
        seat(),
        field_present(&app, seat()),
    );
}

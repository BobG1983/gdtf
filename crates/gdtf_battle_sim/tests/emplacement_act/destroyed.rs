//! Destroying a manned emplacement fans the on-death list of the gun it had mounted.

use bevy::app::App;
use gdtf_battle_sim::{
    effects::{
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            ImmuneArmorTypes,
        },
        on_death::{OnDeathEffect, OnDeathOccurred},
    },
    ganger::Direction,
    metric::CellLevel,
    test_support::{SituationBuilder, TEST_MOUNTED_WEAPON_KEY, emplacement_at, field_turns},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};

use super::harness::*;

/// The seed this case drives.
const SEED: u64 = 0x5543_1243;

/// Ticks the death is given to fan and settle.
const SETTLE_TICKS: u32 = 3;

/// The catalog key the mounted weapon's authored death effect names.
fn wreckage() -> FieldKey {
    FieldKey::new("wreckage".to_owned())
}

/// A registry whose MOUNTED spec authors a death field; the emplacement def authors none.
fn mounted_on_death_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            WeaponSpec {
                on_death: vec![OnDeathEffect::LeaveField { field: wreckage() }],
                ..gun_spec(MOUNT_DAMAGE_TYPE)
            },
        ),
    ])
}

/// The field catalog the effect resolves through; battle setup inserts none of its own.
fn wreckage_catalog() -> FieldDefRegistry {
    FieldDefRegistry::new([(
        wreckage(),
        FieldDef::new(
            FieldDamage::new(3),
            MOUNT_DAMAGE_TYPE,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    )])
}

/// The field standing on a cell, read through the ledger the fan writes into.
fn field_def_at(app: &App, at: CellLevel) -> Option<FieldDef> {
    app.world()
        .get_resource::<FieldRegistry>()
        .and_then(|fields| fields.field_at(&at))
        .map(|placed| placed.def().clone())
}

#[test]
fn a_destroyed_emplacement_fans_its_mounted_guns_on_death_effect() {
    let (mut app, seed) = battle_app(SEED);
    app.insert_resource(mounted_on_death_registry());
    let situation = SituationBuilder::new()
        .with_gangers([player_at(west_entry(), Direction::East)])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    app.insert_resource(wreckage_catalog());
    let emplacement = seated_emplacement(&mut app, seat());
    let actor = player_on(&mut app, west_entry());
    mount(&mut app, actor, emplacement);
    assert!(
        wields_mount(&mut app, actor),
        "PRECONDITION: the seat must have handed out a mount, or there is no mounted gun whose \
         list the destruction below could fan; it wields {}",
        wields_mount(&mut app, actor),
    );
    assert!(
        field_def_at(&app, seat()).is_none(),
        "PRECONDITION: no field stands on the seat before the piece is destroyed; it holds {:?}",
        field_def_at(&app, seat()),
    );

    app.world_mut()
        .write_message(OnDeathOccurred::cover(seat()));
    step(&mut app, SETTLE_TICKS);

    assert!(
        field_def_at(&app, seat()).is_some(),
        "the piece's destruction fans the list on the gun it had mounted, so that LeaveField \
         lands on {:?}; the ledger holds {:?}",
        seat(),
        field_def_at(&app, seat()),
    );
}

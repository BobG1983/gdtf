use bevy::prelude::{
    App, Deref, DerefMut, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut,
    Resource, Update,
};

use crate::{
    effects::{
        bleed::{BleedingOut, tick_bleed},
        dot::tick_dot,
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            ImmuneArmorTypes, tick_fields,
        },
        on_death::{
            CoverOnDeathRegistry, ExplodeDamage, OnDeath, OnDeathEffect, OnDeathOccurred,
            resolve_on_death,
        },
    },
    ganger::{Hp, LifeState, Position, Wounds},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    test_support::{GangerEntityBuilder, dot_turns, field_turns, wield},
    tuning::CombatTuning,
    weapon::{BlastRadius, DamageType, Dot, DotDamage, HitType, Weapon, WieldedBy},
};

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn grid_with_occupant(app: &mut App, cell: CellLevel, occupant: Entity) {
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(cell, Some(occupant));
    app.world_mut().insert_resource(grid);
}

fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

fn dead_ganger_with_on_death(app: &mut App, effect: OnDeathEffect) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .hp(1)
        .life_state(LifeState::Dead)
        .at(ground(5, 5))
        .spawn(app.world_mut());
    wield(app.world_mut(), ganger, (Weapon, OnDeath::new(effect)));
    ganger
}

fn resolver_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<OnDeathOccurred>();
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(FieldRegistry::new());
    app.insert_resource(FieldDefRegistry::default());
    app.insert_resource(CoverOnDeathRegistry::default());
    app.add_systems(Update, resolve_on_death);
    app
}

#[test]
fn explode_on_death_damages_an_adjacent_ganger() {
    let mut app = resolver_app();
    let dead = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        },
    );
    let victim = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(6, 5))))
        .id();
    grid_with_occupant(&mut app, ground(6, 5), victim);

    app.world_mut()
        .write_message(OnDeathOccurred::new(dead, ground(5, 5)));
    app.update();

    assert_eq!(
        hp_of(&app, victim),
        5,
        "the adjacent victim took the blast's flat 5 HP"
    );
    assert_eq!(
        life_of(&app, victim),
        LifeState::Alive,
        "a non-lethal blast leaves the victim alive"
    );
}

#[test]
fn explode_out_of_radius_ganger_is_untouched() {
    let mut app = resolver_app();
    let dead = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        },
    );
    let far = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(10, 10))))
        .id();
    grid_with_occupant(&mut app, ground(10, 10), far);

    app.world_mut()
        .write_message(OnDeathOccurred::new(dead, ground(5, 5)));
    app.update();

    assert_eq!(
        hp_of(&app, far),
        10,
        "a ganger outside the radius is untouched"
    );
}

#[test]
fn leave_field_spawns_the_referenced_field_at_the_death_cell() {
    let mut app = resolver_app();
    let mut defs = FieldDefRegistry::default();
    defs.insert(
        FieldKey::new("burning".to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    );
    app.world_mut().insert_resource(defs);

    let mut cover = CoverOnDeathRegistry::default();
    cover.insert(
        ground(7, 8),
        OnDeathEffect::LeaveField {
            field: FieldKey::new("burning".to_owned()),
        },
    );
    app.world_mut().insert_resource(cover);

    app.world_mut()
        .write_message(OnDeathOccurred::cover(ground(7, 8)));
    app.update();

    let registry = app.world().get_resource::<FieldRegistry>();
    assert!(
        registry.is_some_and(|r| r.field_at(&ground(7, 8)).is_some()),
        "the referenced field was spawned at the death cell"
    );
}

#[test]
fn explode_chain_reaction_kills_then_terminates() {
    let mut app = resolver_app();
    let a = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(100),
            damage_type: DamageType::Blast,
        },
    );
    let b_ganger = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(6, 5))))
        .id();
    app.world_mut().spawn((
        Weapon,
        WieldedBy::new(b_ganger),
        OnDeath::new(OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(100),
            damage_type: DamageType::Blast,
        }),
    ));
    grid_with_occupant(&mut app, ground(6, 5), b_ganger);

    app.world_mut()
        .write_message(OnDeathOccurred::new(a, ground(5, 5)));
    app.update();

    assert_eq!(
        life_of(&app, b_ganger),
        LifeState::Dead,
        "the cascade killed B"
    );
    assert_eq!(hp_of(&app, b_ganger), 0, "B's HP was emptied by A's blast");
}


#[derive(Resource, Default, Deref, DerefMut)]
struct CapturedDeaths(Vec<OnDeathOccurred>);

fn capture_deaths(mut reader: MessageReader<OnDeathOccurred>, mut cap: ResMut<CapturedDeaths>) {
    for death in reader.read() {
        cap.push(*death);
    }
}

fn captured_death_at(app: &App, entity: Entity, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CapturedDeaths>()
        .is_some_and(|c| c.iter().any(|d| d.entity == entity && d.at == at))
}

#[test]
fn tick_dot_emits_on_death_when_a_dot_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::dot::DotTicked>();
    app.add_message::<OnDeathOccurred>();
    app.init_resource::<CapturedDeaths>();
    app.add_systems(Update, (tick_dot, capture_deaths).chain());

    let cell = ground(4, 4);
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(3),
            LifeState::Alive,
            Position::new(cell),
            Dot {
                remaining_turns: dot_turns(2),
                per_turn_damage: DotDamage::new(5),
                damage_type:     DamageType::Plasma,
            },
        ))
        .id();
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the DOT killed the ganger"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_dot emits OnDeathOccurred at the dead ganger's cell on a DOT-kill"
    );
}

#[test]
fn tick_fields_emits_on_death_when_a_field_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::fields::FieldTicked>();
    app.add_message::<OnDeathOccurred>();
    app.add_message::<crate::effects::fields::FieldAfflicted>();
    app.init_resource::<CapturedDeaths>();
    app.add_systems(Update, (tick_fields, capture_deaths).chain());

    let cell = ground(3, 3);
    let ganger = app
        .world_mut()
        .spawn((Hp::new(2), LifeState::Alive, crate::armor::Wears::default()))
        .id();
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(
        cell,
        FieldDef::new(
            FieldDamage::new(5),
            DamageType::Chem,
            ImmuneArmorTypes::default(),
            FieldDuration::Permanent,
        ),
    );
    app.world_mut().insert_resource(registry);
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the field killed the occupant"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_fields emits OnDeathOccurred at the field cell on a field-kill"
    );
}

#[test]
fn tick_bleed_emits_on_death_when_the_wounds_bleed_out_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::bleed::Bleeding>();
    app.add_message::<OnDeathOccurred>();
    app.add_message::<crate::effects::bleed::BleedStarted>();
    app.init_resource::<CapturedDeaths>();
    app.insert_resource(CombatTuning {
        bleed_rate: crate::tuning::BleedRate::new(5),
        ..Default::default()
    });
    app.add_systems(Update, (tick_bleed, capture_deaths).chain());

    let cell = ground(2, 2);
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(10),
            Wounds::new(1),
            LifeState::Downed,
            BleedingOut,
            Position::new(cell),
        ))
        .id();
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the Wounds bleed-out killed the Downed ganger"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_bleed emits OnDeathOccurred at the dead ganger's cell on a bleed-out kill"
    );
}

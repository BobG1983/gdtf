pub(super) use bevy::prelude::{
    App, Deref, DerefMut, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut,
    Resource, Update,
};

pub(super) use crate::{
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::{
            ExplodeDamage, OnDeath, OnDeathEffect, OnDeathOccurred, TerrainOnDeathRegistry,
            resolve_on_death,
        },
    },
    ganger::{Hp, LifeState, Position},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    test_support::{GangerEntityBuilder, wield},
    weapon::{BlastRadius, DamageType, HitType, Weapon},
};

pub(super) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(super) fn grid_with_occupant(app: &mut App, cell: CellLevel, occupant: Entity) {
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(cell, Some(occupant));
    app.world_mut().insert_resource(grid);
}

pub(super) fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

pub(super) fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

pub(super) fn dead_ganger_with_on_death(app: &mut App, effect: OnDeathEffect) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .hp(1)
        .life_state(LifeState::Dead)
        .at(ground(5, 5))
        .spawn(app.world_mut());
    wield(
        app.world_mut(),
        ganger,
        (Weapon, OnDeath::new(vec![effect])),
    );
    ganger
}

pub(super) fn resolver_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<OnDeathOccurred>();
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(FieldRegistry::new());
    app.insert_resource(FieldDefRegistry::default());
    app.insert_resource(TerrainOnDeathRegistry::default());
    app.add_systems(Update, resolve_on_death);
    app
}

#[derive(Resource, Default, Deref, DerefMut)]
pub(super) struct CapturedDeaths(Vec<OnDeathOccurred>);

pub(super) fn capture_deaths(
    mut reader: MessageReader<OnDeathOccurred>,
    mut cap: ResMut<CapturedDeaths>,
) {
    for death in reader.read() {
        cap.push(*death);
    }
}

pub(super) fn captured_death_at(app: &App, entity: Entity, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CapturedDeaths>()
        .is_some_and(|c| c.iter().any(|d| d.entity == entity && d.at == at))
}

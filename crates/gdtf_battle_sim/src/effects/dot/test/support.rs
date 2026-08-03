pub(super) use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
};

pub(super) use crate::{
    effects::dot::{DotAfflicted, DotApplied, DotTicked, apply_dot, tick_dot},
    ganger::{Hp, LifeState, Position},
    weapon::{DamageType, Dot, DotDamage, DotProfile},
};
use crate::{
    metric::{Cell, CellLevel, Level},
    test_support::dot_turns,
};

#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub(super) struct Captured(Vec<DotTicked>);

fn consume(mut reader: MessageReader<DotTicked>, mut captured: ResMut<Captured>) {
    for ticked in reader.read() {
        captured.push(*ticked);
    }
}

pub(super) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(super) fn tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotTicked>();
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_dot, consume).chain());
    app
}

pub(super) fn apply_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotApplied>();
    app.add_message::<DotAfflicted>();
    app.add_systems(Update, apply_dot);
    app
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

pub(super) fn dot_of(app: &App, ganger: Entity) -> Option<Dot> {
    app.world().get::<Dot>(ganger).copied()
}

pub(super) fn tick_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|t| t.ganger == ganger).count())
}

pub(super) fn dot(damage: u16, turns: u8) -> Dot {
    Dot::from_profile(DotProfile::new(
        DotDamage::new(damage),
        DamageType::Kinetic,
        dot_turns(turns),
    ))
}

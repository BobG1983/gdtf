use bevy::{ecs::message::Messages, prelude::App};

use crate::{
    battle::BattleReady,
    metric::{Cell, CellLevel, Level},
    test_support::SimAppBuilder,
};

pub(super) fn headless_app() -> App {
    SimAppBuilder::new().with_battle().with_registries().build()
}

pub(super) fn cl(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

pub(super) fn drain_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

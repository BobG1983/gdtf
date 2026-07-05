//! Shared battle-lifecycle harness helpers for the terrain-entity tests —
//! reached by each concern file via `use super::support::*;`.

use bevy::{ecs::message::Messages, prelude::App};

use crate::{
    battle::BattleReady,
    metric::{Cell, CellLevel, Level},
    test_support::SimAppBuilder,
};

/// A headless app with [`BattleSimPlugin`] — the same harness the battle-lifecycle
/// tests use ([`MinimalPlugins`] + [`AssetPlugin`] + [`ScenePlugin`] + [`BattleSimPlugin`]
/// + the persistent-Load-style resources).
pub(super) fn headless_app() -> App {
    // GTW-576: the composition IS the canonical `with_battle().with_registries()` builder
    // (asset/scene plugins + BattleSimPlugin + the tuning stand-ins + the five test
    // registries the v2 setup_battle resolves against).
    SimAppBuilder::new().with_battle().with_registries().build()
}

/// A helper `(cell, level)` at grid coordinates `(x, y, level)`.
pub(super) fn cl(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Drain [`BattleReady`] messages and return the count.
pub(super) fn drain_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

use bevy::state::state::State;
use gdtf_battle_sim::{
    ganger::{Direction, Facing, Faction},
    metric::CellLevel,
    situation::{GangerSpawn, Situation},
    test_support::{GangerSpawnBuilder, SituationBuilder, key},
};
use gdtf_game::test_support::{BattleAppBuilder, BattleScapeState};

pub(crate) const SHOOTER_FACTION: u8 = 0;
pub(crate) const TARGET_FACTION: u8 = 1;

pub(crate) const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
pub(crate) const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

pub(crate) fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::East))
        .build()
}

pub(crate) fn two_ganger_situation() -> (Situation, Vec<gdtf_battle_sim::situation::PlacedGanger>) {
    let (sx, sy, sl) = SHOOTER_AT;
    let (tx, ty, tl) = TARGET_AT;
    SituationBuilder::new()
        .with_gangers([
            ganger_at(key(sx, sy, sl), SHOOTER_FACTION),
            ganger_at(key(tx, ty, tl), TARGET_FACTION),
        ])
        .build()
}

pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn driven_battle_app() -> bevy::app::App {
    BattleAppBuilder::new()
        .with_situation(two_ganger_situation())
        .build()
}

pub(crate) fn left_battle_running(app: &bevy::app::App) -> bool {
    battlescape_state(app) != Some(BattleScapeState::BattleRunning)
}

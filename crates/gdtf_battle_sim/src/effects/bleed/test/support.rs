use bevy::prelude::Messages;
pub(super) use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
};

use crate::{
    acts::EndTurnRequested,
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, insert_sim_resources},
    turn::ActiveFaction,
};
pub(super) use crate::{
    effects::bleed::{Bleeding, BleedingOut, tick_bleed},
    ganger::{Faction, LifeState, Wounds},
    tuning::CombatTuning,
};

#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub(super) struct Captured(Vec<Bleeding>);

fn consume(mut reader: MessageReader<Bleeding>, mut captured: ResMut<Captured>) {
    for bled in reader.read() {
        captured.push(*bled);
    }
}

pub(super) fn bleed_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<Bleeding>();
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    app.add_message::<crate::effects::bleed::BleedStarted>();
    app.init_resource::<CombatTuning>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_bleed, consume).chain());
    app
}

pub(super) fn bleed_rate() -> u8 {
    *CombatTuning::default().bleed_rate
}

pub(super) fn wounds_of(app: &App, ganger: Entity) -> u8 {
    app.world().get::<Wounds>(ganger).map_or(0, |w| **w)
}

pub(super) fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

pub(super) fn bleeding_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|b| b.ganger == ganger).count())
}


pub(super) const SEED: u64 = 0x5A1C_AC75;

pub(super) const PLAYER: Faction = Faction::new(0);
pub(super) const ENEMY: Faction = Faction::new(1);

pub(super) fn seed_battle_resources(app: &mut App) {
    insert_sim_resources(app, BattleSeed::new(SEED));
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
}

pub(super) fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    seed_battle_resources(&mut app);
    app.insert_resource(BattleInProgress);
    app
}

pub(super) fn bleeding_ganger(
    app: &mut App,
    faction: Faction,
    life: LifeState,
    wounds: u8,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .faction(faction)
        .life_state(life)
        .wounds(wounds)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    if life == LifeState::Downed {
        app.world_mut().entity_mut(ganger).insert(BleedingOut);
    }
    ganger
}

pub(super) fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
}

pub(super) fn drain_bleeding(app: &mut App) -> Vec<Bleeding> {
    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .drain()
        .collect()
}

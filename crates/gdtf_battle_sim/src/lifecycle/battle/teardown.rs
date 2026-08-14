//! Handle [`TeardownBattleRequested`]: despawn terrain and clear battle resources.

use bevy::prelude::{Commands, MessageReader};

use crate::{
    act_log::ActLog,
    battle::{
        messages::TeardownBattleRequested,
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    rng::{
        BattleSeed, FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng,
    },
    slab::SlabLedger,
    surface::SurfaceGrid,
    terrain::{entity::TerrainIndex, floor::FloorCostGrid},
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::{OmniscientFog, SquadVisibility},
};

fn remove_rng_streams(commands: &mut bevy::prelude::Commands) {
    commands.remove_resource::<BattleSeed>();
    commands.remove_resource::<ShotRng>();
    commands.remove_resource::<SeverityRng>();
    commands.remove_resource::<LootRng>();
    commands.remove_resource::<InjuryRng>();
    commands.remove_resource::<ProcgenRng>();
    commands.remove_resource::<ReactionRng>();
    commands.remove_resource::<FightRng>();
}

/// Tear down an active battle when requested.
pub fn teardown_battle_on_request(
    mut requests: MessageReader<TeardownBattleRequested>,
    terrain_entities: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<crate::terrain::entity::TerrainCell>,
    >,
    mut commands: Commands,
) {
    let mut requested = false;
    for _request in requests.read() {
        requested = true;
    }
    if requested {
        remove_rng_streams(&mut commands);
        commands.remove_resource::<CoverLedger>();
        commands.remove_resource::<SurfaceGrid>();
        commands.remove_resource::<SlabLedger>();
        commands.remove_resource::<crate::slab::BraceStairCells>();
        commands.remove_resource::<OccupancyGrid>();
        commands.remove_resource::<VerticalLinkGraph>();
        commands.remove_resource::<TerrainIndex>();
        commands.remove_resource::<FloorCostGrid>();
        commands.remove_resource::<crate::effects::fields::FieldRegistry>();
        commands.remove_resource::<crate::effects::on_death::CoverOnDeathRegistry>();
        for entity in terrain_entities.iter() {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<BattleInProgress>();
        commands.remove_resource::<PlayerFaction>();
        commands.remove_resource::<BattleRoster>();
        commands.remove_resource::<ActiveFaction>();
        commands.remove_resource::<ActLog>();
        commands.remove_resource::<SquadVisibility>();
        commands.remove_resource::<OmniscientFog>();
    }
}

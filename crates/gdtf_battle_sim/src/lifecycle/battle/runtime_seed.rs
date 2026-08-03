use bevy::prelude::Commands;

use crate::{
    act_log::ActLog,
    battle::{
        messages::SetupBattleRequested,
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    occupancy::OccupancyGrid,
    rng::{FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng},
    turn::ActiveFaction,
    visibility::{OmniscientFog, SquadVisibility},
};

/// NOTE: the [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid) is inserted by
pub(super) fn insert_battle_runtime(commands: &mut Commands, request: &SetupBattleRequested) {
    let root = request.seed;
    commands.insert_resource(ShotRng::from_root(root));
    commands.insert_resource(SeverityRng::from_root(root));
    commands.insert_resource(LootRng::from_root(root));
    commands.insert_resource(InjuryRng::from_root(root));
    commands.insert_resource(ProcgenRng::from_root(root));
    commands.insert_resource(ReactionRng::from_root(root));
    commands.insert_resource(FightRng::from_root(root));

    commands.insert_resource(BattleInProgress);
    commands.insert_resource(PlayerFaction::new(request.situation.player_faction));
    commands.insert_resource(BattleRoster::new(
        request
            .situation
            .gangers
            .iter()
            .map(|ganger| ganger.faction),
    ));
    commands.insert_resource(ActiveFaction::new(request.situation.player_faction));
    commands.insert_resource(ActLog::default());
    commands.insert_resource(SquadVisibility::default());
    commands.insert_resource(OmniscientFog::new(SquadVisibility::omniscient(
        &OccupancyGrid::new(),
    )));
}

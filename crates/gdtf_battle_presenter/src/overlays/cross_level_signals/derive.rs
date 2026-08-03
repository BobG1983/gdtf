use bevy::{ecs::system::SystemParam, platform::collections::HashSet, prelude::*};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, Faction, Level, LifeState, OccupancyGrid, Position},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

use super::{
    aggregate::build_signals, connector::gather_connectors, drop_depth::gather_drop_depth,
    threat::gather_threats, types::CrossLevelSignals,
};
use crate::{ActiveLevel, TerrainSprite};

#[derive(SystemParam)]
pub struct CrossLevelSimFacts<'w, 's> {
        active:    Res<'w, ActiveLevel>,
            squad:     Option<Res<'w, SquadVisibility>>,
            player:    Option<Res<'w, PlayerFaction>>,
        gangers:   Query<'w, 's, (&'static Position, &'static Faction, &'static LifeState)>,
        surface:   Option<Res<'w, SurfaceGrid>>,
        occupancy: Option<Res<'w, OccupancyGrid>>,
        graph:     Option<Res<'w, VerticalLinkGraph>>,
            terrain:   Query<'w, 's, &'static TerrainSprite>,
}

pub fn derive_cross_level_signals(
    mut signals: ResMut<CrossLevelSignals>,
    facts: CrossLevelSimFacts,
) {
    let Some(squad) = facts.squad.as_deref() else {
        signals.set_if_neq(CrossLevelSignals::default());
        return;
    };
    let active_level: Level = **facts.active;

    let threats = gather_threats(
        facts.gangers.iter(),
        squad,
        facts.player.as_deref().copied(),
        active_level,
    );

    let drops = match (facts.surface.as_deref(), facts.occupancy.as_deref()) {
        (Some(surface), Some(occupancy)) => {
            let drawn = drawn_cells_on(active_level, &facts.terrain);
            gather_drop_depth(active_level, surface, occupancy, &drawn, squad)
        }
        _ => Vec::new(),
    };
    let connectors = match facts.graph.as_deref() {
        Some(graph) => gather_connectors(active_level, graph, squad),
        None => Vec::new(),
    };

    signals.set_if_neq(build_signals(threats, drops, connectors));
}

fn drawn_cells_on(active_level: Level, terrain: &Query<&TerrainSprite>) -> HashSet<Cell> {
    terrain
        .iter()
        .filter(|sprite| sprite.at.level() == active_level)
        .map(|sprite| sprite.at.cell())
        .collect()
}

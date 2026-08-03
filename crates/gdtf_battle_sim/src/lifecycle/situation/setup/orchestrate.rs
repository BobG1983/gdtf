use bevy::prelude::Commands;

use super::{
    registries::{BattleRegistries, BattleSetup},
    resolve, seed_cover, seed_slabs, spawn_gangers,
};
use crate::{
    occupancy::{OccupancyGrid, OccupancyInput, TerrainPlacement},
    situation::{BattleSetupError, Situation},
    terrain::{entity::TerrainIndex, floor::FloorCostGrid},
    tuning::MoveCost,
    vertical::build_vertical_link_graph,
};

/// Resolve situation data and spawn the battle world resources.
///
/// # Errors
///
/// Returns [`BattleSetupError`] when vertical links are invalid, roster/weapon/armor
/// resolution fails, gangers share a cell, or field/cover setup cannot complete.
pub fn setup_battle(
    situation: &Situation,
    registries: BattleRegistries<'_>,
    fallback_floor_cost: MoveCost,
    commands: &mut Commands,
) -> Result<BattleSetup, BattleSetupError> {
    let BattleRegistries {
        gangs,
        weapons,
        melee_weapons,
        armor,
        stat_tuning,
        terrain,
        fields,
        attachments,
    } = registries;
    let vertical_graph = build_vertical_link_graph(situation)?;

    let resolved_members = resolve::resolve_members(situation, gangs)?;

    if let Some(at) = resolve::first_stacked_cell(situation) {
        return Err(BattleSetupError::StackedGangers { at });
    }

    let weapon_bundles = resolve::resolve_weapon_bundles(&resolved_members, weapons, attachments)?;
    let melee_bundles =
        resolve::resolve_melee_bundles(&resolved_members, melee_weapons, attachments)?;
    let armor_specs = resolve::resolve_armor_specs(&resolved_members, armor)?;
    let (resolved_covers, cover_on_death_entries) = resolve::resolve_covers(situation, terrain)?;
    let resolved_slabs = resolve::resolve_slabs(situation, terrain)?;

    let field_registry = resolve::build_field_registry(situation, fields)?;

    let (default_floor_cost, floor_overrides) = (fallback_floor_cost, Vec::new());

    let occupants = spawn_gangers::spawn_gangers(
        commands,
        &resolved_members,
        weapon_bundles,
        melee_bundles,
        armor_specs,
        stat_tuning,
    );

    let (mut terrain_pairs, occupancy_kinds) =
        seed_cover::seed_cover_terrain(situation, resolved_covers, commands);

    let (stair_cell_set, brace_stair_cells_set) = seed_slabs::stair_cell_sets(situation);

    terrain_pairs.extend(seed_slabs::seed_slab_terrain(
        situation,
        &resolved_slabs,
        &brace_stair_cells_set,
        commands,
    ));

    let setup = BattleSetup { occupants };

    let terrain_placements: Vec<TerrainPlacement> = situation
        .walls
        .iter()
        .chain(situation.scatter.iter())
        .zip(occupancy_kinds)
        .map(|(cover, kind)| TerrainPlacement::new(cover.at, kind))
        .collect();
    let occupancy_input = OccupancyInput {
        terrain:   terrain_placements,
        occupants: setup.occupants.clone(),
    };
    let occupancy_grid =
        OccupancyGrid::build_from_occupancy_input(&occupancy_input, &stair_cell_set);
    commands.insert_resource(occupancy_grid);

    commands.insert_resource(TerrainIndex::new(terrain_pairs));

    commands.insert_resource(FloorCostGrid::new(default_floor_cost, floor_overrides));

    commands.insert_resource(vertical_graph);

    commands.insert_resource(field_registry);

    commands.insert_resource(crate::effects::on_death::CoverOnDeathRegistry::new(
        cover_on_death_entries,
    ));

    Ok(setup)
}

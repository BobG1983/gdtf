//! Live battles shaped so the shooter has exactly one emplacement to take, or is already in it.

use bevy::{
    app::App,
    ecs::{entity::Entity, relationship::Relationship},
};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    acts::{downed::is_8_adjacent, exit_emplacement_tu_cost},
    emplacement::{
        EmplacementEntrySides, EmplacementFacing, EmplacementState, MountedBy, SetEmplacement,
    },
    entity::TerrainCell,
    ganger::{LifeState, Position, Tu},
    occupancy::OccupancyGrid,
    prelude::CellLevel,
    terrain::facing::TerrainFacing,
    tuning::CombatTuning,
};
use gdtf_game::qa_wire::cell::CellLevelNet;
use gdtf_test_utils::advance_until;

use crate::{
    battle_reads::{clear_cells_away_from, one_cardinal_step_from, player_gangers},
    contextual_acts::scene::{place, select_a_player_ganger, settle},
    socket_support::{TestError, battle_app_listening},
};

/// Who is manning an emplacement right now.
pub(crate) fn occupant_of(app: &App, emplacement: Entity) -> Option<Entity> {
    app.world()
        .get::<MountedBy>(emplacement)
        .map(Relationship::get)
}

/// Every vacant emplacement the generated map already put 8-adjacent to `at`.
fn vacant_emplacements_around(app: &App, at: CellLevel) -> Vec<Entity> {
    app.world()
        .iter_entities()
        .filter_map(|entity| {
            let state = *entity.get::<EmplacementState>()?;
            let cell = **entity.get::<TerrainCell>()?;
            (!*state.is_occupied() && *is_8_adjacent(Position::new(at), Position::new(cell)))
                .then_some(entity.id())
        })
        .collect()
}

/// Move the map's own vacant emplacements out of reach, so only ours can be offered.
fn clear_emplacements_around(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let crowd = vacant_emplacements_around(app, at);
    let mut away = clear_cells_away_from(app, at).into_iter();
    for emplacement in crowd {
        let Some(cell) = away.next() else {
            return Err(
                "the map must hold a clear cell out of reach for each emplacement it \
                        started beside the shooter"
                    .into(),
            );
        };
        let Ok(mut row) = app.world_mut().get_entity_mut(emplacement) else {
            return Err("the emplacement the world just answered with must still exist".into());
        };
        row.insert(TerrainCell::new(cell));
    }
    Ok(())
}

/// What the emplacement fixture set up: who, which seat, and the two cells the shooter uses.
pub(crate) struct EmplacementScene {
    /// The player ganger the fixture selected.
    pub(crate) shooter:     Entity,
    /// The seat the fixture spawned beside it.
    pub(crate) emplacement: Entity,
    /// The seat's own cell, which the shooter ends up standing on once it enters.
    pub(crate) seat:        CellLevel,
    /// The cell the shooter enters from, and is put back on when it leaves.
    pub(crate) entry:       CellLevel,
}

/// A live battle whose selected shooter has exactly one vacant emplacement beside it: ours.
pub(crate) fn emplacement_beside_the_shooter() -> Result<(App, McpPort, EmplacementScene), TestError>
{
    let (mut app, port) = battle_app_listening()?;
    let (shooter, at) = select_a_player_ganger(&mut app)?;
    clear_emplacements_around(&mut app, at)?;
    let Some(beside) = one_cardinal_step_from(&app, CellLevelNet::from_sim(at)) else {
        return Err(
            "the generated map must offer one clear CARDINAL cell beside the shooter, or the \
             emplacement stands on no entry side of its own"
                .into(),
        );
    };
    let emplacement = app
        .world_mut()
        .spawn((
            TerrainCell::new(beside.to_sim()),
            EmplacementState::Vacant,
            EmplacementEntrySides::new(TerrainFacing::ALL.to_vec()),
            EmplacementFacing::new(TerrainFacing::default()),
        ))
        .id();
    settle(&mut app);
    Ok((
        app,
        port,
        EmplacementScene {
            shooter,
            emplacement,
            seat: beside.to_sim(),
            entry: at,
        },
    ))
}

/// Fail unless the shooter came out of the enter exchange able to be offered the dismount.
fn survived_the_enter_exchange(app: &App, shooter: Entity) -> Result<(), TestError> {
    if !matches!(
        app.world().get::<LifeState>(shooter),
        Some(LifeState::Alive)
    ) {
        return Err(
            "the enter exchange downed the shooter, so the selection is cleared and the \
                    Exit offer is gone — the fixture, not a missing offer"
                .into(),
        );
    }
    let Some(tuning) = app.world().get_resource::<CombatTuning>() else {
        return Err("a running battle must hold the tuning the dismount is priced from".into());
    };
    let cost = exit_emplacement_tu_cost(tuning);
    let Some(pool) = app.world().get::<Tu>(shooter).copied() else {
        return Err("the selected shooter must carry a Tu pool".into());
    };
    if *pool < *cost {
        return Err(format!(
            "the enter exchange drained the shooter to {} TU, under the {} the dismount costs, \
             so can_exit_emplacement refuses — the fixture, not a missing offer",
            *pool, *cost,
        )
        .into());
    }
    Ok(())
}

/// The same battle with the shooter already manning that emplacement, through the sim's toggle.
pub(crate) fn manned_emplacement_under_the_shooter()
-> Result<(App, McpPort, EmplacementScene), TestError> {
    let (mut app, port, scene) = emplacement_beside_the_shooter()?;
    let (shooter, emplacement) = (scene.shooter, scene.emplacement);
    app.world_mut()
        .write_message(SetEmplacement::occupy(emplacement, shooter));
    advance_until(&mut app, |app| {
        occupant_of(app, emplacement) == Some(shooter)
    });
    settle(&mut app);
    survived_the_enter_exchange(&app, shooter)?;
    Ok((app, port, scene))
}

/// Who the occupancy grid names on a cell right now.
fn occupant_at(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|grid| grid.occupant(&at))
}

/// The manned battle with a second player ganger standing on the cell the shooter entered from.
pub(crate) fn a_held_entry_cell_under_the_manned_emplacement()
-> Result<(App, McpPort, EmplacementScene), TestError> {
    let (mut app, port, scene) = manned_emplacement_under_the_shooter()?;
    let Some(other) = player_gangers(&app)
        .into_iter()
        .find(|ganger| *ganger != scene.shooter)
    else {
        return Err(
            "the battle must field a second living player ganger to stand on the cell the \
             shooter entered from"
                .into(),
        );
    };
    place(&mut app, other, scene.entry)?;
    settle(&mut app);
    if occupant_at(&app, scene.entry) != Some(other) {
        return Err(format!(
            "the grid must name the second ganger on the cell the shooter entered from at {:?}; \
             it names {:?} — the fixture, not a missing offer",
            scene.entry,
            occupant_at(&app, scene.entry),
        )
        .into());
    }
    Ok((app, port, scene))
}

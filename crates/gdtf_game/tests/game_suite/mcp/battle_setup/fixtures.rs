//! Battle fixtures that write world state, so a read has something real to answer with.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_input::{ChosenFireMode, SelectedShooter};
use gdtf_battle_presenter::DrawnPosition;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct, WatchingFactions},
    battle::{BattleInProgress, PlayerFaction},
    entity::TerrainCell,
    ganger::{Direction, Facing, Faction, GangerName, HpMax, Position, Tu, TuMax},
    openable::OpenState,
    prelude::{Cell, CellLevel},
    turn::ActiveFaction,
    weapon::{FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};
use gdtf_game::qa_wire::{
    cell::CellLevelNet,
    misc::ModeKindNet,
    roster::{FactionNet, GangerNameNet},
    token::GangerToken,
    vitals::{HpMaxNet, TuMaxNet},
};

use super::{
    catch_up::let_the_screen_catch_up,
    expected::{
        ExpectedEnemy, ExpectedFireMode, ExpectedTurn, FLOODED_LOG_LINES, IdlePair,
        LOG_LINES_WRITTEN, LitCover, LiveCard, LoggedActor, MagazineLoad, PosedShooter,
        SpawnedDoor, Standing,
    },
    map::{a_free_open_cell, a_free_open_cell_beside, a_lit_cover_cell, settle},
};
use crate::mcp::{
    battle_fixture::arm_selected_with_modes,
    battle_reads::{
        a_player_ganger, an_enemy_faction, an_enemy_ganger, an_enemy_ganger_at,
        an_unselected_player_ganger, cell_of,
    },
    magazine_support::{empty_the_magazine, fill_the_magazine},
    socket_support::{TestError, battle_app_listening},
};

/// A live battle with one enemy standing where the squad can or cannot see it.
pub(crate) fn battle_with_an_enemy(
    standing: Standing,
) -> Result<(App, McpPort, ExpectedEnemy), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some(at) = a_free_open_cell(&app, standing) else {
        return Err(format!("the generated map must offer a free open cell {standing:?}").into());
    };
    stand_at(&mut app, entity, at, at)?;
    let at = CellLevelNet::from_sim(at);
    Ok((app, port, ExpectedEnemy { entity, at }))
}

/// Move a ganger, sim and screen together, the way a settled battle holds it.
pub(super) fn stand_at(
    app: &mut App,
    entity: Entity,
    live: CellLevel,
    drawn: CellLevel,
) -> Result<(), TestError> {
    let Ok(mut row) = app.world_mut().get_entity_mut(entity) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert((
        Position::new(live),
        DrawnPosition::seeded(Position::new(drawn)),
    ));
    Ok(())
}

/// A live battle with a living enemy stood beside a player ganger the game has not selected.
pub(crate) fn battle_with_an_enemy_beside_an_idle_ganger()
-> Result<(App, McpPort, IdlePair), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(shooter) = an_unselected_player_ganger(&app) else {
        return Err("a generated battle must field a player ganger the game has not picked".into());
    };
    let Some(at) = cell_of(&app, shooter) else {
        return Err("the ganger the world just answered with must be standing somewhere".into());
    };
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let away_from: Vec<CellLevel> = selected_cell(&app).into_iter().collect();
    let Some(next_to) = a_free_open_cell_beside(&app, at.to_sim(), &away_from) else {
        return Err(
            "the generated map must offer a free open cell beside the idle ganger, out \
                    of reach of the ganger the game selected"
                .into(),
        );
    };
    stand_at(&mut app, enemy, next_to, next_to)?;
    let_the_screen_catch_up(&mut app)?;
    Ok((app, port, IdlePair { shooter, enemy }))
}

/// The cell the ganger the game selected for itself is standing on.
fn selected_cell(app: &App) -> Option<CellLevel> {
    let selected = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|shooter| **shooter)?;
    cell_of(app, selected).map(CellLevelNet::to_sim)
}

/// A live battle, reporting the first living enemy and where it is deployed.
pub(crate) fn battle_reporting_an_enemy() -> Result<(App, McpPort, ExpectedEnemy), TestError> {
    let (app, port) = battle_app_listening()?;
    let Some((entity, at)) = an_enemy_ganger_at(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    Ok((app, port, ExpectedEnemy { entity, at }))
}

/// A live battle with a door in `state` standing where the squad can or cannot see it.
pub(crate) fn battle_with_a_door(
    standing: Standing,
    state: OpenState,
) -> Result<(App, McpPort, SpawnedDoor), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(at) = a_free_open_cell(&app, standing) else {
        return Err(format!("the generated map must offer a free open cell {standing:?}").into());
    };
    let entity = app.world_mut().spawn((TerrainCell::new(at), state)).id();
    let at = CellLevelNet::from_sim(at);
    Ok((app, port, SpawnedDoor { entity, at }))
}

/// A live battle whose selected shooter's gun offers single and full, and is set to full.
pub(crate) fn battle_with_a_selected_fire_mode()
-> Result<(App, McpPort, ExpectedFireMode), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let modes = FireMode::new(vec![
        one_mode(ModeKind::Single, 0.2),
        one_mode(ModeKind::Full, 0.9),
    ]);
    let weapon = arm_selected_with_modes(&mut app, modes);
    let Ok(mut gun) = app.world_mut().get_entity_mut(weapon) else {
        return Err("the weapon the fixture just armed must still exist".into());
    };
    gun.insert(ChosenFireMode::new(ModeKind::Full));
    settle(&mut app);
    let kind = ModeKindNet::from_sim(ModeKind::Full);
    Ok((app, port, ExpectedFireMode { kind }))
}

/// One fire-mode entry, at the kind and price the fixture gives it.
const fn one_mode(kind: ModeKind, tu_percent: f32) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(1),
    )
}

/// A live battle, reporting one player ganger's identity as the world holds it.
pub(crate) fn battle_reporting_a_player_card() -> Result<(App, McpPort, LiveCard), TestError> {
    let (app, port) = battle_app_listening()?;
    let Some((entity, _)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    let Ok(row) = app.world().get_entity(entity) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    let (Some(faction), Some(tu_max)) = (row.get::<Faction>(), row.get::<TuMax>()) else {
        return Err("a deployed ganger carries a gang and a TU maximum".into());
    };
    let card = LiveCard {
        token:   GangerToken::new(entity.to_bits()),
        name:    row
            .get::<GangerName>()
            .map(|name| GangerNameNet::new((**name).clone())),
        faction: FactionNet::from_sim(*faction),
        tu_max:  TuMaxNet::new(**tu_max),
        hp_max:  row.get::<HpMax>().map(|max| HpMaxNet::new(**max)),
    };
    Ok((app, port, card))
}

/// A live battle, reporting a wall or cover cell that stands inside the lit area.
pub(crate) fn battle_with_lit_cover() -> Result<(App, McpPort, LitCover), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(at) = a_lit_cover_cell(&app) else {
        return Err(
            "a squad deployed inside a generated map always sees some wall or cover; the lit \
             area held none"
                .into(),
        );
    };
    Ok((app, port, LitCover { at }))
}

/// A live battle whose act log already holds [`LOG_LINES_WRITTEN`] lines for one ganger.
pub(crate) fn battle_with_log_lines() -> Result<(App, McpPort, LoggedActor), TestError> {
    battle_with_a_log_of(LOG_LINES_WRITTEN)
}

/// A live battle whose act log holds [`FLOODED_LOG_LINES`] lines, past any default window.
pub(crate) fn battle_with_a_flooded_log() -> Result<(App, McpPort, LoggedActor), TestError> {
    battle_with_a_log_of(FLOODED_LOG_LINES)
}

fn battle_with_a_log_of(lines: u32) -> Result<(App, McpPort, LoggedActor), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let Some((actor, _)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    let Some(player) = app
        .world()
        .get_resource::<PlayerFaction>()
        .map(|gang| **gang)
    else {
        return Err("a running battle names the gang the player commands".into());
    };
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        return Err("a running battle carries the sim's act log".into());
    };
    for _ in 0..lines {
        log.append(RecordedAct::new(
            actor,
            ActProvenance::Commanded,
            ActDeed::TurnBegan {
                now_active: Faction::new(0),
            },
            watched_by(player),
        ));
    }
    Ok((app, port, LoggedActor { actor }))
}

/// Witnesses recording one gang as having seen the act and being able to name its actor.
pub(crate) fn watched_by(gang: Faction) -> ActWitnesses {
    let watching = WatchingFactions::new([gang]);
    ActWitnesses::new(watching.clone(), watching)
}

/// A live battle whose selected shooter faces north holding `tu` time units and `load` rounds.
pub(crate) fn battle_with_a_shooter_facing_north(
    tu: Tu,
    load: MagazineLoad,
) -> Result<(App, McpPort, PosedShooter), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let Some((shooter, at)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    let rounds = match load {
        MagazineLoad::Loaded => fill_the_magazine(&mut app, shooter),
        MagazineLoad::Empty => empty_the_magazine(&mut app, shooter),
    };
    if rounds.is_none() {
        return Err("the posed shooter must hold a ranged weapon with a magazine".into());
    }
    let Ok(mut row) = app.world_mut().get_entity_mut(shooter) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert((Facing::new(Direction::North), tu));
    let (cell, level) = at.to_sim().split();
    let behind = CellLevel::new(Cell::new(cell.x, cell.y + 1), level);
    let behind = CellLevelNet::from_sim(behind);
    Ok((app, port, PosedShooter { behind }))
}

/// A live battle with an enemy gang set as the acting one, so the two turn gangs differ.
pub(crate) fn battle_with_another_gang_acting() -> Result<(App, McpPort, ExpectedTurn), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(player) = app
        .world()
        .get_resource::<PlayerFaction>()
        .map(|gang| **gang)
    else {
        return Err("a running battle names the gang the player commands".into());
    };
    let Some(acting) = an_enemy_faction(&app) else {
        return Err("a generated battle must field a gang other than the player's".into());
    };
    hold_the_sim_still(&mut app)?;
    app.world_mut().insert_resource(ActiveFaction::new(acting));
    let turn = ExpectedTurn {
        active: FactionNet::from_sim(acting),
        player: FactionNet::from_sim(player),
    };
    Ok((app, port, turn))
}

/// Stop the sim clock, so the turn cycle cannot move on while a read is in flight.
fn hold_the_sim_still(app: &mut App) -> Result<(), TestError> {
    if app
        .world_mut()
        .remove_resource::<BattleInProgress>()
        .is_none()
    {
        return Err("a running battle carries the sim's in-progress marker".into());
    }
    Ok(())
}

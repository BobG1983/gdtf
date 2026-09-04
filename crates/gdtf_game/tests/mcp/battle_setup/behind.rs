//! Fixtures where the screen is a step behind the sim, so the two can be told apart.

use bevy::{app::App, platform::collections::HashSet};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_presenter::{ActHold, PlaybackCursor, ShownSquadVisibility};
use gdtf_battle_sim::{
    prelude::{CellLevel, OccupancyGrid},
    visibility::SquadVisibility,
};
use gdtf_game::qa_wire::cell::CellLevelNet;

use super::{
    expected::{FrozenFog, LateOccupant, RememberedCover, SplitEnemy, SplitMover, Standing},
    fixtures::stand_at,
    map::{a_free_open_cell, a_lit_cover_cell, settle, shown_fog},
};
use crate::{
    battle_reads::{a_player_ganger, an_enemy_ganger},
    socket_support::{TestError, battle_app_listening},
};

/// Longer than any case takes, so the gate is still shut when the reply goes out.
const GATE_HOLD_SECONDS: f32 = 600.0;

/// A battle where the sim has moved an enemy but the screen still draws it where it was.
pub(crate) fn battle_with_a_ganger_the_screen_has_not_moved(
    drawn: Standing,
) -> Result<(App, McpPort, SplitEnemy), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let (Some(drawn_at), Some(live_at)) = (
        a_free_open_cell(&app, drawn),
        a_free_open_cell(&app, drawn.opposite()),
    ) else {
        return Err("the generated map must offer a free open cell on each side of the fog".into());
    };
    hold_the_screen_still(&mut app)?;
    stand_at(&mut app, entity, live_at, drawn_at)?;
    Ok((
        app,
        port,
        SplitEnemy {
            entity,
            drawn: CellLevelNet::from_sim(drawn_at),
            live: CellLevelNet::from_sim(live_at),
        },
    ))
}

/// A battle where the sim has moved a player ganger but the screen still draws it where it was.
///
/// The drawn cell stands inside the area the screen lights and the live cell outside it, so a
/// read planned from one answers differently from a read planned from the other.
pub(crate) fn battle_with_a_player_ganger_the_screen_has_not_moved()
-> Result<(App, McpPort, SplitMover), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some((entity, _)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    let (Some(drawn_at), Some(live_at)) = (
        a_free_open_cell(&app, Standing::Lit),
        a_free_open_cell(&app, Standing::Hidden),
    ) else {
        return Err("the generated map must offer a free open cell on each side of the fog".into());
    };
    hold_the_screen_still(&mut app)?;
    stand_at(&mut app, entity, live_at, drawn_at)?;
    Ok((
        app,
        port,
        SplitMover {
            entity,
            drawn: drawn_at,
            live: live_at,
        },
    ))
}

/// A battle whose screen still lights a cell the sim's own fog leaves dark.
pub(crate) fn battle_with_a_frozen_fog() -> Result<(App, McpPort, FrozenFog), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some(at) = a_free_open_cell(&app, Standing::Hidden) else {
        return Err("the generated map must offer a free open cell outside the lit area".into());
    };
    hold_the_screen_still(&mut app)?;
    light_on_screen_only(&mut app, at)?;
    stand_at(&mut app, entity, at, at)?;
    Ok((
        app,
        port,
        FrozenFog {
            entity,
            at: CellLevelNet::from_sim(at),
        },
    ))
}

/// A battle whose screen has stopped lighting a cover cell its own fog still remembers.
pub(crate) fn battle_with_remembered_cover() -> Result<(App, McpPort, RememberedCover), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(at) = a_lit_cover_cell(&app) else {
        return Err(
            "a squad deployed inside a generated map always sees some wall or cover; the lit \
             area held none"
                .into(),
        );
    };
    hold_the_screen_still(&mut app)?;
    darken_on_screen_only(&mut app, at.to_sim())?;
    Ok((app, port, RememberedCover { at }))
}

/// A battle where the sim has put a ganger on a cell the screen has not shown it reaching.
pub(crate) fn battle_with_an_occupant_the_screen_has_not_seen()
-> Result<(App, McpPort, LateOccupant), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some(at) = a_free_open_cell(&app, Standing::Lit) else {
        return Err("the generated map must offer a free open cell inside the lit area".into());
    };
    hold_the_screen_still(&mut app)?;
    let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
        return Err("a running battle carries the sim's occupancy grid".into());
    };
    grid.set_occupant(at, Some(entity));
    Ok((
        app,
        port,
        LateOccupant {
            at: CellLevelNet::from_sim(at),
        },
    ))
}

/// Hold the playback cursor, which shuts the gate and freezes every shadow the screen draws.
pub(crate) fn hold_the_screen_still(app: &mut App) -> Result<(), TestError> {
    let Some(mut cursor) = app.world_mut().get_resource_mut::<PlaybackCursor>() else {
        return Err("a battle presenter carries the playback cursor".into());
    };
    cursor.hold_for(ActHold::timed(GATE_HOLD_SECONDS));
    Ok(())
}

/// Drop one cell out of the screen's frozen fog entirely, so it is neither lit nor remembered.
///
/// `darken_on_screen_only` leaves the cell explored, and a route is planned through anything
/// the fog still remembers, so only forgetting it takes the cell off a reachable set.
pub(crate) fn forget_on_screen_only(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let Some(fog) = shown_fog(app) else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    let mut visible: HashSet<CellLevel> = fog.visible_cells().copied().collect();
    let mut explored: HashSet<CellLevel> = fog.explored_cells().copied().collect();
    if !explored.remove(&at) {
        return Err("the screen's fog must remember the chosen cell for the case to bite".into());
    }
    visible.remove(&at);
    let forgotten = SquadVisibility::new(visible, explored);
    let Some(mut shadow) = app.world_mut().get_resource_mut::<ShownSquadVisibility>() else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    shadow.promote(&forgotten);
    Ok(())
}

/// Drop one cell from the screen's frozen fog, leaving it in what that fog remembers.
fn darken_on_screen_only(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let Some(fog) = shown_fog(app) else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    let mut visible: HashSet<CellLevel> = fog.visible_cells().copied().collect();
    let explored: HashSet<CellLevel> = fog.explored_cells().copied().collect();
    if !visible.remove(&at) {
        return Err("the screen must be lighting the chosen cell for the case to bite".into());
    }
    if !explored.contains(&at) {
        return Err("the screen's fog must remember the chosen cell it stops lighting".into());
    }
    let remembered = SquadVisibility::new(visible, explored);
    let Some(mut shadow) = app.world_mut().get_resource_mut::<ShownSquadVisibility>() else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    shadow.promote(&remembered);
    Ok(())
}

/// Light one more cell in the screen's frozen fog, leaving the sim's own fog dark there.
fn light_on_screen_only(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let Some(live) = app.world().get_resource::<SquadVisibility>() else {
        return Err("a running battle carries the sim's squad fog".into());
    };
    if *live.is_cell_visible(&at) {
        return Err("the sim's fog must leave the chosen cell dark for the case to bite".into());
    }
    let Some(fog) = shown_fog(app) else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    let mut visible: HashSet<CellLevel> = fog.visible_cells().copied().collect();
    let mut explored: HashSet<CellLevel> = fog.explored_cells().copied().collect();
    visible.insert(at);
    explored.insert(at);
    let lit = SquadVisibility::new(visible, explored);
    let Some(mut shadow) = app.world_mut().get_resource_mut::<ShownSquadVisibility>() else {
        return Err("the presenter carries the fog the screen draws".into());
    };
    shadow.promote(&lit);
    Ok(())
}

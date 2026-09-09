//! Fixtures that author one cell's terrain, so a case can name a kind the map may not hold.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    emplacement::{EmplacementState, MountedWeaponKey},
    entity::{TerrainCell, TerrainPieceKind},
    occupancy::{OccupancyGrid, TerrainKind},
    prelude::CellLevel,
    visibility::CellVisible,
    weapon::WeaponName,
};
use gdtf_game::qa_wire::cell::CellLevelNet;

use super::{
    expected::Standing,
    map::{a_free_open_cell, a_lit_cover_cell, settle, shown_fog},
};
use crate::mcp::{
    contextual_acts::emplacement::manned_emplacement_under_the_shooter,
    socket_support::{TestError, battle_app_listening},
};

/// The weapon these fixtures mount on the emplacement they author.
const MOUNTED_GUN: &str = "Heavy Stubber";

/// The cell a fixture authored terrain on.
#[derive(Debug)]
pub(crate) struct AuthoredCell {
    pub(crate) at: CellLevelNet,
}

/// A manned emplacement seat, and the ganger riding it.
#[derive(Debug)]
pub(crate) struct MannedSeat {
    pub(crate) shooter: Entity,
    pub(crate) at:      CellLevelNet,
}

/// Whether the fog the screen draws lights a cell.
pub(crate) fn shown_fog_lights(app: &App, at: CellLevel) -> CellVisible {
    shown_fog(app).map_or_else(|| CellVisible::new(false), |fog| fog.is_cell_visible(&at))
}

/// A battle whose lit cover cell has been authored into an emplacement, ledger entry and all.
pub(crate) fn battle_with_a_lit_emplacement() -> Result<(App, McpPort, AuthoredCell), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(at) = a_lit_cover_cell(&app) else {
        return Err(
            "a squad deployed inside a generated map always sees some wall or cover; the lit \
             area held none"
                .into(),
        );
    };
    author_a_seat(&mut app, at.to_sim())?;
    settle(&mut app);
    Ok((app, port, AuthoredCell { at }))
}

/// A battle with a lit wall cell the cover ledger holds no entry for.
pub(crate) fn battle_with_a_lit_unledgered_wall() -> Result<(App, McpPort, AuthoredCell), TestError>
{
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(at) = a_free_open_cell(&app, Standing::Lit) else {
        return Err("the generated map must offer a free open cell inside the lit area".into());
    };
    if ledger_entry(&app, at).is_some() {
        return Err("an open floor cell must hold no cover entry for this case to bite".into());
    }
    let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
        return Err("a running battle carries the sim's occupancy grid".into());
    };
    grid.set_terrain(at, TerrainKind::Wall);
    settle(&mut app);
    Ok((
        app,
        port,
        AuthoredCell {
            at: CellLevelNet::from_sim(at),
        },
    ))
}

/// The manned emplacement, with its seat authored into the grid and ledger as a map's is.
pub(crate) fn manned_emplacement_on_authored_terrain()
-> Result<(App, McpPort, MannedSeat), TestError> {
    let (mut app, port, manned) = manned_emplacement_under_the_shooter()?;
    let (shooter, emplacement) = (manned.shooter, manned.emplacement);
    let Some(at) = app
        .world()
        .get::<TerrainCell>(emplacement)
        .map(|cell| **cell)
    else {
        return Err("the fixture's emplacement stands on a cell of its own".into());
    };
    author_a_seat(&mut app, at)?;
    settle(&mut app);
    Ok((
        app,
        port,
        MannedSeat {
            shooter,
            at: CellLevelNet::from_sim(at),
        },
    ))
}

/// Make `at` read as a generated map's emplacement: grid kind, ledger entry, seat components.
fn author_a_seat(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let seat = match terrain_entity_at(app, at) {
        Some(entity) => entity,
        None => app.world_mut().spawn(TerrainCell::new(at)).id(),
    };
    let Ok(mut row) = app.world_mut().get_entity_mut(seat) else {
        return Err("the terrain entity the world just answered with must still exist".into());
    };
    if row.get::<EmplacementState>().is_none() {
        row.insert(EmplacementState::Vacant);
    }
    row.insert(MountedWeaponKey::new(WeaponName::new(
        MOUNTED_GUN.to_owned(),
    )));
    let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
        return Err("a running battle carries the sim's occupancy grid".into());
    };
    grid.set_terrain(at, TerrainKind::Emplacement);
    if ledger_entry(app, at).is_none() {
        let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
            return Err("a running battle carries the sim's cover ledger".into());
        };
        ledger.insert(at, a_seat_entry());
    }
    Ok(())
}

const fn a_seat_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(8),
        HeightBand::Mid,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
        TerrainPieceKind::Emplacement,
    )
}

fn ledger_entry(app: &App, at: CellLevel) -> Option<CoverEntry> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at).copied())
}

fn terrain_entity_at(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .iter_entities()
        .find_map(|entity| (**entity.get::<TerrainCell>()? == at).then_some(entity.id()))
}

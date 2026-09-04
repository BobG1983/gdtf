//! Shaping a live battle so one contextual act family, and only that one, is on offer.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::message::McpResponse;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    cover::HeightBand,
    entity::TerrainCell,
    ganger::{Position, Tu, TuMax},
    openable::{OpenState, OpenableBlocking, SetOpenable},
    prelude::{Cell, CellLevel},
};
use gdtf_game::{
    qa_wire::{
        act::{ActRefusalNet, ActSeqNet},
        act_payload::MeleeTargetNet,
        cell::CellLevelNet,
        offer::OfferTargetNet,
    },
    test_support::ContextualReply,
};
use gdtf_test_utils::advance_until;

use super::super::{
    act_support::decode,
    battle_reads::{
        clear_cells_away_from, enemies_beside, one_step_from, player_gangers, two_steps_from,
    },
    socket_support::TestError,
};

/// Frames a refused act is given to prove it moved nothing — per-frame sim work, no IO.
pub(crate) const HOLD_FRAMES: u32 = 64;

/// What an accepted contextual reply carries, as the cases read it.
pub(crate) struct Accepted {
    pub(crate) from_seq: ActSeqNet,
    pub(crate) to_seq:   ActSeqNet,
    pub(crate) target:   OfferTargetNet,
}

/// The window and target an accepted reply carries, or a failure naming the refusal.
pub(crate) fn accepted(name: &'static str, reply: McpResponse) -> Result<Accepted, TestError> {
    match decode::<ContextualReply>(name, reply)? {
        ContextualReply::Accepted {
            from_seq,
            to_seq,
            target,
            ..
        } => Ok(Accepted {
            from_seq,
            to_seq,
            target,
        }),
        ContextualReply::Refused { reason } => {
            Err(format!("`{name}` must be accepted here, it was refused: {reason:?}").into())
        }
    }
}

/// Why a refused reply was turned away, or a failure naming the acceptance.
pub(crate) fn refused(name: &'static str, reply: McpResponse) -> Result<ActRefusalNet, TestError> {
    match decode::<ContextualReply>(name, reply)? {
        ContextualReply::Refused { reason } => Ok(reason),
        ContextualReply::Accepted { target, .. } => {
            Err(format!("`{name}` must be refused here, it fired at {target:?}").into())
        }
    }
}

/// Render a melee target as the compact RON body `act.melee` takes.
pub(crate) fn melee_argument(target: MeleeTargetNet) -> String {
    let Ok(text) = ron::ser::to_string(&target) else {
        unreachable!("a wire melee target serializes to compact RON");
    };
    format!("(target:{text})")
}

/// Select the first living player ganger and report where it stands.
pub(crate) fn select_a_player_ganger(app: &mut App) -> Result<(Entity, CellLevel), TestError> {
    let Some(shooter) = player_gangers(app).into_iter().next() else {
        return Err("a generated battle must field at least one living player ganger".into());
    };
    let Some(at) = position_of(app, shooter) else {
        return Err("the player ganger the world just answered with must stand somewhere".into());
    };
    if let Some(max) = app.world().get::<TuMax>(shooter).copied()
        && let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter)
    {
        *tu = Tu::new(*max);
    }
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    Ok((shooter, at))
}

/// Who the fixture selected before it handed the battle over.
pub(crate) fn shooter(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)
}

/// Where a ganger stands right now.
pub(crate) fn position_of(app: &App, ganger: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(ganger).map(|at| **at)
}

/// Move a ganger onto a cell, the way the fixtures place a neighbour.
pub(crate) fn place(app: &mut App, ganger: Entity, at: CellLevel) -> Result<(), TestError> {
    let Ok(mut row) = app.world_mut().get_entity_mut(ganger) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert(Position::new(at));
    Ok(())
}

/// A clear cell one step from `at`.
pub(crate) fn a_neighbour(app: &App, at: CellLevel) -> Result<CellLevel, TestError> {
    one_step_from(app, CellLevelNet::from_sim(at))
        .map(CellLevelNet::to_sim)
        .ok_or_else(|| "the generated map must offer one clear cell beside the shooter".into())
}

/// A clear cell one step from `at`, with the clear cell a shove from `at` would push it on to.
pub(crate) fn a_shovable_neighbour(
    app: &App,
    at: CellLevel,
) -> Result<(CellLevel, CellLevel), TestError> {
    let Some(second) = two_steps_from(app, CellLevelNet::from_sim(at)) else {
        return Err(
            "the generated map must offer two clear cells in a line from the shooter".into(),
        );
    };
    let (from, level) = at.split();
    let landing = second.to_sim();
    let (to, _) = landing.split();
    let beside = CellLevel::new(
        Cell::new(from.x + (to.x - from.x) / 2, from.y + (to.y - from.y) / 2),
        level,
    );
    Ok((beside, landing))
}

/// Move the map's own enemies out of the shooter's reach, so only `keep` can be offered.
pub(crate) fn clear_enemies_around(
    app: &mut App,
    at: CellLevel,
    keep: Entity,
) -> Result<(), TestError> {
    let crowd: Vec<Entity> = enemies_beside(app, at)
        .into_iter()
        .filter(|enemy| *enemy != keep)
        .collect();
    let mut away = clear_cells_away_from(app, at).into_iter();
    for enemy in crowd {
        let Some(cell) = away.next() else {
            return Err(
                "the map must hold a clear cell out of reach for each enemy it \
                        started beside the shooter"
                    .into(),
            );
        };
        place(app, enemy, cell)?;
    }
    if enemies_beside(app, at) == vec![keep] {
        return Ok(());
    }
    Err(
        "only the enemy this fixture placed may stand beside the shooter, or the panel's scan \
         picks by map layout rather than by what the case set up"
            .into(),
    )
}

/// Spawn a closed door on a cell, carrying what the offer scan and the sim both read.
pub(crate) fn spawn_closed_door(app: &mut App, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(HeightBand::High),
        ))
        .id()
}

/// Whether a door is open right now.
pub(crate) fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
}

/// Every closed door the generated map already put 8-adjacent to `at`.
pub(crate) fn closed_doors_around(app: &App, at: CellLevel) -> Vec<Entity> {
    app.world()
        .iter_entities()
        .filter_map(|entity| {
            let state = *entity.get::<OpenState>()?;
            let cell = **entity.get::<TerrainCell>()?;
            (state == OpenState::Closed && *is_8_adjacent(Position::new(at), Position::new(cell)))
                .then_some(entity.id())
        })
        .collect()
}

/// Open the map's own adjacent doors through the real toggle, so only ours can be offered.
pub(crate) fn clear_doors_around(app: &mut App, at: CellLevel) {
    for door in closed_doors_around(app, at) {
        app.world_mut().write_message(SetOpenable::open(door));
    }
    advance_until(app, |app| closed_doors_around(app, at).is_empty());
}

/// Run the app until whatever the fixture just changed has been taken up by the sim.
pub(crate) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

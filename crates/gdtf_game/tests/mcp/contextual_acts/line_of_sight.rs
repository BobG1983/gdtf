//! `act.melee` over a real socket when the sim's sight probe refuses the strike: the panel falls
//! back to a wall, so naming the ganger is refused and naming that wall is struck.

use bevy::{
    app::{App, Update},
    ecs::{entity::Entity, resource::Resource},
    prelude::{MessageReader, ResMut},
};
use cobalt_mcp_protocol::{
    command::RunOptions,
    message::{McpRequest, McpResponse},
    ports::McpPort,
};
use cobalt_test_utils::advance_until;
use gdtf_battle_sim::{
    acts::MeleeResolved,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{LifeState, Position},
    prelude::CellLevel,
};
use gdtf_game::{
    qa_wire::{
        act::ActRefusalNet,
        act_payload::MeleeTargetNet,
        cell::CellLevelNet,
        offer::{ContextualActNet, ContextualOfferNet, OfferTargetNet},
    },
    test_support::MCP_PROTOCOL_VERSION,
};
use serde::Deserialize;

use super::{
    super::{
        act_support::{assert_caught_up, caught_up, decode, next},
        battle_reads::{a_clear_diagonal_from, an_enemy_ganger, token_of},
        command_exchange::{ACT_MELEE, BATTLE_OFFERS, WAIT, exchange_inspecting, run},
        socket_support::{Client, TestError, TestResult, battle_app_listening},
    },
    scene::{
        HOLD_FRAMES, accepted, clear_enemies_around, melee_argument, place, refused,
        select_a_player_ganger, settle,
    },
};

/// The strike the case lined up: who swings, at whom, and the corners walled between them.
pub(super) struct BlockedStrike {
    actor:   Entity,
    enemy:   Entity,
    corners: [CellLevel; 2],
}

/// A wall tall enough to break the line between two diagonal neighbours.
const fn a_high_wall() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(100),
        HeightBand::High,
        ArmorProtection::new(50),
        ArmorHardness::new(50),
        TerrainPieceKind::Wall,
    )
}

/// A live battle with the enemy diagonally beside the shooter and both shared corners walled.
fn enemy_behind_a_walled_corner() -> Result<(App, McpPort, BlockedStrike), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (actor, at) = select_a_player_ganger(&mut app)?;
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some(diagonal) = a_clear_diagonal_from(&app, CellLevelNet::from_sim(at)) else {
        return Err(
            "the generated map must offer one clear diagonal cell beside the shooter".into(),
        );
    };
    place(&mut app, enemy, diagonal.step)?;
    clear_enemies_around(&mut app, at, enemy)?;
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        return Err("a running battle must hold the cover ledger the walls go into".into());
    };
    for corner in diagonal.corners {
        ledger.insert(corner, a_high_wall());
    }
    settle(&mut app);
    Ok((
        app,
        port,
        BlockedStrike {
            actor,
            enemy,
            corners: diagonal.corners,
        },
    ))
}

/// Every cell the sim has resolved a structure strike on since the case started.
#[derive(Resource, Default)]
struct StruckCells {
    cells: Vec<CellLevel>,
}

fn record_struck_cells(mut resolved: MessageReader<MeleeResolved>, mut log: ResMut<StruckCells>) {
    for strike in resolved.read() {
        log.cells.push(strike.at);
    }
}

/// Start recording structure strikes, before the fixture takes its first request.
fn watch_struck_cells(app: &mut App) {
    app.init_resource::<StruckCells>();
    app.add_systems(Update, record_struck_cells);
}

/// Whether the sim has resolved a structure strike on `at`.
fn struck_at(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<StruckCells>()
        .is_some_and(|log| log.cells.contains(&at))
}

/// The body `battle.offers` answers with, as far as this case reads it.
#[derive(Debug, Deserialize)]
struct OffersBody {
    offers: Vec<ContextualOfferNet>,
}

/// The cell the panel's melee button is offering, or a failure naming what it offered instead.
fn offered_cell(body: &OffersBody) -> Result<CellLevelNet, TestError> {
    let Some(offer) = body
        .offers
        .iter()
        .find(|offer| offer.act == ContextualActNet::Melee)
    else {
        return Err(format!(
            "the shooter stands beside the walled corner, so the panel must be offering a melee \
             button: {body:?}"
        )
        .into());
    };
    match offer.target {
        OfferTargetNet::Cell(at) => Ok(at),
        named => Err(format!(
            "the sight probe leaves the panel falling back to a structure cell, so the melee \
             offer must name one: {named:?}"
        )
        .into()),
    }
}

/// Whether both walled corners still hold standing cover that has taken no damage.
fn walls_untouched(app: &App, corners: &[CellLevel; 2]) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .is_some_and(|ledger| {
            corners.iter().all(|corner| {
                ledger
                    .peek(corner)
                    .is_some_and(|entry| !*entry.destroyed && entry.current_hp == entry.max_hp)
            })
        })
}

/// The enemy must still be alive and still standing diagonally beside the actor.
fn assert_the_pair_held(app: &App, strike: &BlockedStrike) {
    let world = app.world();
    let alive = world
        .get_entity(strike.enemy)
        .is_ok_and(|row| row.get::<LifeState>().is_none_or(|life| *life.is_active()));
    assert!(
        alive,
        "the target must still be alive when the reply landed, or the answer was decided by \
         something else",
    );
    let beside = world
        .get_entity(strike.actor)
        .ok()
        .and_then(|row| row.get::<Position>().copied())
        .zip(
            world
                .get_entity(strike.enemy)
                .ok()
                .and_then(|row| row.get::<Position>().copied()),
        )
        .is_some_and(|(actor, enemy)| {
            (actor.cell().x - enemy.cell().x).abs() == 1
                && (actor.cell().y - enemy.cell().y).abs() == 1
        });
    assert!(
        beside,
        "the two must still be diagonal neighbours when the reply landed, or the refusal came \
         from reach rather than from the panel offering something else",
    );
}

#[test]
fn a_melee_call_naming_the_target_behind_the_walled_corner_is_refused_not_redirected() -> TestResult
{
    let (mut app, replies, strike) = exchange_inspecting(enemy_behind_a_walled_corner, |strike| {
        let target = MeleeTargetNet::Ganger(token_of(strike.enemy));
        vec![
            caught_up(),
            run(ACT_MELEE, &melee_argument(target), RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let reason = refused(ACT_MELEE, next(ACT_MELEE, &mut replies)?)?;

    assert_the_pair_held(&app, &strike);
    assert!(
        walls_untouched(&app, &strike.corners),
        "both seeded walls must still stand when the reply landed, or the refusal came from \
         something else",
    );
    assert_eq!(
        reason,
        ActRefusalNet::TargetMismatch,
        "the sight probe leaves the panel offering the corner wall, so a call naming the ganger \
         the quote priced names a target that is not on offer",
    );
    for _ in 0..HOLD_FRAMES {
        app.update();
    }
    assert!(
        walls_untouched(&app, &strike.corners),
        "a refused call pushes nothing, so the corner wall the panel was offering must never be \
         hit; it lost HP within {HOLD_FRAMES} frames",
    );
    Ok(())
}

#[test]
fn a_melee_call_naming_the_offered_structure_cell_is_accepted_and_struck() -> TestResult {
    let (mut app, port, strike) = enemy_behind_a_walled_corner()?;
    watch_struck_cells(&mut app);

    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &McpRequest::Hello(MCP_PROTOCOL_VERSION))?;
    if !matches!(hello, McpResponse::HelloOk(_)) {
        return Err(format!("the handshake must succeed first, got {hello:?}").into());
    }
    assert_caught_up(client.exchange(&mut app, &caught_up())?)?;

    let listed = client.exchange(&mut app, &run(BATTLE_OFFERS, "()", RunOptions::default()))?;
    let at = offered_cell(&decode::<OffersBody>(BATTLE_OFFERS, listed)?)?;
    let called = client.exchange(
        &mut app,
        &run(
            ACT_MELEE,
            &melee_argument(MeleeTargetNet::Structure(at)),
            RunOptions::default(),
        ),
    )?;
    let swung = accepted(ACT_MELEE, called)?;

    assert_the_pair_held(&app, &strike);
    assert_eq!(
        swung.target,
        OfferTargetNet::Cell(at),
        "a call naming the structure cell the panel is offering is the call the panel would \
         make, so it is accepted and the reply names that same cell back",
    );
    advance_until(&mut app, |app| struck_at(app, at.to_sim()));
    Ok(())
}

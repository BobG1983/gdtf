//! What `battle.cost` quotes for a strike at a cell: refused at bare ground, legal at cover.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::Tu,
    occupancy::{OccupancyGrid, TerrainKind},
    prelude::CellLevel,
};
use gdtf_game::qa_wire::{
    act_payload::MeleeTargetNet,
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
};

use super::support::{CostBody, cost_calls, first_body, settle};
use crate::mcp::{
    battle_reads::{a_player_ganger, one_step_from},
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// More time units than any strike costs, so nothing but the reach gate can refuse the quote.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

/// What the case leaves standing on the cell it asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ground {
    /// Left as the map generated it, which is open ground.
    Bare,
    /// A cover piece put in the ledger and on the grid.
    Standing,
}

/// A cover piece tough enough that no settling frame can knock it down.
const fn a_cover_piece() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(1_000),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    )
}

/// Put a cover piece at `at`, in the ledger a smash depletes and the grid it stands on.
fn stand_cover_at(app: &mut App, at: CellLevel) -> Option<()> {
    app.world_mut()
        .get_resource_mut::<CoverLedger>()?
        .insert(at, a_cover_piece());
    app.world_mut()
        .get_resource_mut::<OccupancyGrid>()?
        .set_terrain(at, TerrainKind::Cover);
    Some(())
}

/// Who swings, and the neighbouring cell the case asks the price of smashing.
struct Smash {
    actor: Entity,
    at:    CellLevel,
}

/// Find a player ganger and a clear cell beside it, standing cover on that cell when asked.
fn line_up(app: &mut App, ground: Ground) -> Option<Smash> {
    settle(app);
    let (actor, at) = a_player_ganger(app)?;
    let beside = one_step_from(app, at)?.to_sim();
    app.world_mut().get_entity_mut(actor).ok()?.insert(AMPLE_TU);
    if ground == Ground::Standing {
        stand_cover_at(app, beside)?;
    }
    settle(app);
    Some(Smash { actor, at: beside })
}

/// The reply to one `battle.cost {Melee, Structure}` at that cell, with the world it left.
fn quote_the_smash(ground: Ground) -> Result<(App, Smash, CostBody), TestError> {
    let mut planned: Option<Smash> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(smash) = line_up(app, ground) else {
            return Vec::new();
        };
        let calls = cost_calls(
            smash.actor,
            &[CostActNet::Melee {
                target: MeleeTargetNet::Structure(CellLevelNet::from_sim(smash.at)),
            }],
        );
        planned = Some(smash);
        calls
    })?;
    let Some(smash) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with one clear cell beside it to swing \
             at",
        ));
    };
    let body = first_body(replies)?;
    Ok((app, smash, body))
}

#[test]
fn a_structure_strike_at_bare_ground_is_quoted_not_allowed() -> TestResult {
    let (app, smash, body) = quote_the_smash(Ground::Bare)?;
    assert!(
        app.world()
            .get_resource::<CoverLedger>()
            .is_some_and(|ledger| ledger.peek(&smash.at).is_none()),
        "this case only means something while the target cell holds no ledger entry, and \
         something put one there",
    );
    assert_eq!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .map(|grid| grid.terrain(&smash.at)),
        Some(TerrainKind::Open),
        "this case only means something while the target cell holds no terrain either",
    );
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "the sim smashes nothing at a cell holding neither cover nor terrain, so the quote must \
         refuse the strike rather than price one that resolves to nothing: {body:?}",
    );
    assert!(!*body.legal, "a refused strike is not legal: {body:?}");
    assert!(
        body.cost.is_some(),
        "the refusal must be the reach gate's: an actor holding no melee weapon is refused the \
         same way and quotes no cost at all: {body:?}",
    );
    Ok(())
}

#[test]
fn a_structure_strike_at_standing_cover_is_quoted_legal() -> TestResult {
    let (app, smash, body) = quote_the_smash(Ground::Standing)?;
    assert!(
        app.world()
            .get_resource::<CoverLedger>()
            .is_some_and(|ledger| ledger.peek(&smash.at).is_some()),
        "the cover the case stood must still be in the ledger when the reply landed, or the \
         verdict was decided by something else",
    );
    assert_eq!(
        body.refusal, None,
        "a strike at standing cover is one the sim resolves, so the quote carries no refusal: \
         {body:?}",
    );
    assert!(*body.legal, "a strike the sim resolves is legal: {body:?}");
    Ok(())
}

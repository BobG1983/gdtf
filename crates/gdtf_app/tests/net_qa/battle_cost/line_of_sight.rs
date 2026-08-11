//! What the sim's line-of-sight probe does to a melee quote: refused when high cover fills both
//! shared corners of a diagonal, quoted legal when those corners are clear.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    act_payload::MeleeTargetNet,
    cost::{CostActNet, CostRefusalNet},
    token::GangerToken,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{LifeState, Position, Tu},
    prelude::CellLevel,
};

use super::support::{cost_calls, first_body, settle};
use crate::{
    battle_reads::{a_clear_diagonal_from, a_player_ganger, an_enemy_ganger},
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// More time units than any strike costs, so nothing but the gate can refuse the quote.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

/// Whether the case walls off the diagonal's corners before it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Corners {
    /// Left as the map generated them.
    Clear,
    /// Filled with high cover on both cells.
    Walled,
}

/// The strike the case lined up: who swings, at whom, and the corners it squeezes past.
struct Strike {
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
    )
}

/// Stand an enemy diagonally beside a player ganger, walling the shared corners when asked.
fn line_up(app: &mut App, corners: Corners) -> Option<Strike> {
    settle(app);
    let (actor, at) = a_player_ganger(app)?;
    let enemy = an_enemy_ganger(app)?;
    let diagonal = a_clear_diagonal_from(app, at)?;
    app.world_mut()
        .get_entity_mut(enemy)
        .ok()?
        .insert(Position::new(diagonal.step));
    app.world_mut().get_entity_mut(actor).ok()?.insert(AMPLE_TU);
    settle(app);
    if corners == Corners::Walled {
        let mut ledger = app.world_mut().get_resource_mut::<CoverLedger>()?;
        for corner in diagonal.corners {
            ledger.insert(corner, a_high_wall());
        }
    }
    Some(Strike {
        actor,
        enemy,
        corners: diagonal.corners,
    })
}

/// The reply to one `battle.cost {Melee}` against the enemy, with what the world still holds.
fn quote_the_strike(
    corners: Corners,
) -> Result<(App, Strike, super::support::CostBody), TestError> {
    let mut planned: Option<Strike> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(strike) = line_up(app, corners) else {
            return Vec::new();
        };
        let calls = cost_calls(
            strike.actor,
            &[CostActNet::Melee {
                target: MeleeTargetNet::Ganger(GangerToken::new(strike.enemy.to_bits())),
            }],
        );
        planned = Some(strike);
        calls
    })?;
    let Some(strike) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with a clear diagonal cell to stand an \
             enemy on",
        ));
    };
    let body = first_body(replies)?;
    Ok((app, strike, body))
}

/// The enemy must still be alive and still standing diagonally beside the actor.
fn assert_the_pair_held(app: &App, strike: &Strike) {
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
        "the two must still be diagonal neighbours when the reply landed, or the answer was \
         decided by reach rather than by sight",
    );
}

#[test]
fn a_strike_at_a_diagonal_neighbour_with_clear_corners_is_quoted_legal() -> TestResult {
    let (app, strike, body) = quote_the_strike(Corners::Clear)?;
    assert_the_pair_held(&app, &strike);
    assert!(
        app.world()
            .get_resource::<CoverLedger>()
            .is_some_and(|ledger| strike
                .corners
                .iter()
                .all(|corner| ledger.peek(corner).is_none_or(|entry| *entry.destroyed))),
        "this case only means something while both corners are open, and the generated map put \
         standing cover on one of them",
    );
    assert_eq!(
        body.refusal, None,
        "a strike the sim would let land carries no refusal: {body:?}",
    );
    assert!(
        *body.legal,
        "a strike the sim would let land is legal: {body:?}"
    );
    Ok(())
}

#[test]
fn a_strike_the_line_of_sight_probe_refuses_is_quoted_no_line_of_sight() -> TestResult {
    let (app, strike, body) = quote_the_strike(Corners::Walled)?;
    assert_the_pair_held(&app, &strike);
    assert!(
        app.world()
            .get_resource::<CoverLedger>()
            .is_some_and(|ledger| strike
                .corners
                .iter()
                .all(|corner| ledger.peek(corner).is_some_and(|entry| !*entry.destroyed))),
        "both seeded walls must still stand when the reply landed, or the refusal came from \
         something else",
    );
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::NoLineOfSight),
        "cover on both shared corners is what stops the sim's own strike, so the quote must \
         refuse NoLineOfSight rather than price a strike that cannot land: {body:?}",
    );
    assert!(!*body.legal, "a refused strike is not legal: {body:?}");
    Ok(())
}

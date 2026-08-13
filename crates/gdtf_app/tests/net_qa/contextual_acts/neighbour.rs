//! The neighbour acts over a real socket: the panel picks who, and the sim carries it out.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{act_payload::MeleeTargetNet, offer::OfferTargetNet, token::GangerToken};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position, Tu, TuMax},
    prelude::CellLevel,
};
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};
use gdtf_test_utils::advance_until;

use super::{
    super::{
        act_support::{assert_caught_up, caught_up, next},
        battle_reads::{an_enemy_ganger, an_unselected_player_ganger, token_of},
        command_exchange::{
            ACT_EXECUTE, ACT_MELEE, ACT_SHOVE, ACT_STABILIZE, WAIT, exchange_inspecting, run,
        },
        socket_support::{TestError, TestResult, battle_app_listening},
    },
    scene::{
        SETTLE_BUDGET, a_neighbour, a_shovable_neighbour, accepted, clear_enemies_around,
        melee_argument, place, position_of, select_a_player_ganger, settle, shooter,
    },
};

/// A live battle with one living enemy beside the shooter and clear ground behind it.
pub(super) fn enemy_beside_the_shooter() -> Result<(App, NetQaPort, (Entity, CellLevel)), TestError>
{
    let (mut app, port) = battle_app_listening()?;
    let (_shooter, at) = select_a_player_ganger(&mut app)?;
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let (beside, landing) = a_shovable_neighbour(&app, at)?;
    place(&mut app, enemy, beside)?;
    clear_enemies_around(&mut app, at, enemy)?;
    settle(&mut app);
    Ok((app, port, (enemy, landing)))
}

/// A live battle with one downed enemy beside the shooter and no other enemy in reach.
pub(super) fn downed_enemy_beside_the_shooter() -> Result<(App, NetQaPort, Entity), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (_shooter, at) = select_a_player_ganger(&mut app)?;
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let beside = a_neighbour(&app, at)?;
    place(&mut app, enemy, beside)?;
    clear_enemies_around(&mut app, at, enemy)?;
    let Ok(mut row) = app.world_mut().get_entity_mut(enemy) else {
        return Err("the enemy the world just answered with must still exist".into());
    };
    row.insert(LifeState::Downed);
    settle(&mut app);
    Ok((app, port, enemy))
}

/// Every downed, bleeding ganger of `gang` standing 8-adjacent to `at`.
fn bleeding_gang_mates_beside(app: &App, at: CellLevel, gang: Faction) -> Vec<Entity> {
    app.world()
        .iter_entities()
        .filter_map(|entity| {
            let faction = *entity.get::<Faction>()?;
            let position = *entity.get::<Position>()?;
            let life = *entity.get::<LifeState>()?;
            (faction == gang
                && life == LifeState::Downed
                && entity.get::<BleedingOut>().is_some()
                && *is_8_adjacent(Position::new(at), position))
            .then_some(entity.id())
        })
        .collect()
}

/// A live battle with one downed, bleeding gang mate beside the shooter and no other.
pub(super) fn bleeding_mate_beside_the_shooter() -> Result<(App, NetQaPort, Entity), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (shooter, at) = select_a_player_ganger(&mut app)?;
    let Some(gang) = app.world().get::<Faction>(shooter).copied() else {
        return Err("the selected shooter must fight for a gang".into());
    };
    let Some(mate) = an_unselected_player_ganger(&app) else {
        return Err("a generated battle must field a second living player ganger".into());
    };
    let beside = a_neighbour(&app, at)?;
    place(&mut app, mate, beside)?;
    let Ok(mut row) = app.world_mut().get_entity_mut(mate) else {
        return Err("the gang mate the world just answered with must still exist".into());
    };
    row.insert((LifeState::Downed, BleedingOut));
    settle(&mut app);
    if bleeding_gang_mates_beside(&app, at, gang) != vec![mate] {
        return Err(
            "only the gang mate this fixture downed may be bleeding beside the shooter, or the \
             panel's scan picks by map layout rather than by what the case set up"
                .into(),
        );
    }
    Ok((app, port, mate))
}

/// What `ganger` has left to spend, with the pool its stat block allows.
fn pool_of(app: &App, ganger: Entity) -> Option<(Tu, TuMax)> {
    app.world()
        .get_entity(ganger)
        .ok()
        .and_then(|row| row.get::<Tu>().copied().zip(row.get::<TuMax>().copied()))
}

#[test]
fn a_melee_call_naming_the_offered_neighbour_is_accepted_and_swings_at_it() -> TestResult {
    let (mut app, replies, (enemy, _landing)) =
        exchange_inspecting(enemy_beside_the_shooter, |(enemy, _landing)| {
            let target = MeleeTargetNet::Ganger(token_of(*enemy));
            vec![
                caught_up(),
                run(ACT_MELEE, &melee_argument(target), RunOptions::default()),
            ]
        })?;
    let Some(actor) = shooter(&app) else {
        return Err("the fixture must hand over a battle with a shooter selected".into());
    };
    if pool_of(&app, actor).is_none() {
        return Err(
            "the selected shooter must carry both a TU pool and its maximum, or a pool this \
             case spent cannot be told from one it never had"
                .into(),
        );
    }
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let struck = accepted(ACT_MELEE, next(ACT_MELEE, &mut replies)?)?;

    assert_eq!(
        struck.target,
        OfferTargetNet::Ganger(GangerToken::new(enemy.to_bits())),
        "a call that names the ganger the panel is offering is the call the panel would make, \
         so it is accepted and the reply names that same ganger back",
    );
    assert!(
        struck.to_seq > struck.from_seq,
        "a strike the sim carried out is recorded, so the window it opened is not empty: {:?}..{:?}",
        struck.from_seq,
        struck.to_seq,
    );
    let charged = advance_until(
        &mut app,
        |app| pool_of(app, actor).is_some_and(|(tu, max)| *tu < *max),
        SETTLE_BUDGET,
    );
    assert!(
        charged,
        "the sim's own melee charges the attacker for the strike, so the full pool the fixture \
         wrote must be spent down within {SETTLE_BUDGET} frames; it still held {:?}",
        pool_of(&app, actor),
    );
    Ok(())
}

#[test]
fn shoving_the_offered_neighbour_names_it_and_pushes_it_off_its_cell() -> TestResult {
    let (mut app, replies, (enemy, landing)) =
        exchange_inspecting(enemy_beside_the_shooter, |_placed| {
            vec![caught_up(), run(ACT_SHOVE, "()", RunOptions::default())]
        })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let shoved = accepted(ACT_SHOVE, next(ACT_SHOVE, &mut replies)?)?;

    assert_eq!(
        shoved.target,
        OfferTargetNet::Ganger(GangerToken::new(enemy.to_bits())),
        "the reply names the ganger the panel was offering, which is the one the call fired at",
    );
    assert!(
        shoved.to_seq > shoved.from_seq,
        "a shove the sim carried out is recorded, so the window it opened is not empty: {:?}..{:?}",
        shoved.from_seq,
        shoved.to_seq,
    );
    let pushed = advance_until(
        &mut app,
        |app| position_of(app, enemy).map(|at| at.cell()) == Some(landing.cell()),
        SETTLE_BUDGET,
    );
    assert!(
        pushed,
        "the sim's own shove must push the target one cell on along the line it was shoved \
         from; it wanted {:?} and last stood at {:?}",
        landing.cell(),
        position_of(&app, enemy),
    );
    Ok(())
}

#[test]
fn executing_the_offered_downed_enemy_kills_it_in_the_world() -> TestResult {
    let (mut app, replies, enemy) =
        exchange_inspecting(downed_enemy_beside_the_shooter, |_enemy| {
            vec![caught_up(), run(ACT_EXECUTE, "()", RunOptions::default())]
        })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let executed = accepted(ACT_EXECUTE, next(ACT_EXECUTE, &mut replies)?)?;

    assert_eq!(
        executed.target,
        OfferTargetNet::Ganger(GangerToken::new(enemy.to_bits())),
        "the reply names the downed ganger the panel was offering",
    );
    let killed = advance_until(
        &mut app,
        |app| app.world().get::<LifeState>(enemy) == Some(&LifeState::Dead),
        SETTLE_BUDGET,
    );
    assert!(
        killed,
        "the sim's own execute must leave the carried target Dead; last life state was {:?}",
        app.world().get::<LifeState>(enemy),
    );
    Ok(())
}

#[test]
fn stabilizing_the_offered_gang_mate_stops_its_bleed_rather_than_finishing_it() -> TestResult {
    let (mut app, replies, mate) =
        exchange_inspecting(bleeding_mate_beside_the_shooter, |_mate| {
            vec![caught_up(), run(ACT_STABILIZE, "()", RunOptions::default())]
        })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let stabilized = accepted(ACT_STABILIZE, next(ACT_STABILIZE, &mut replies)?)?;

    assert_eq!(
        stabilized.target,
        OfferTargetNet::Ganger(GangerToken::new(mate.to_bits())),
        "the reply names the bleeding gang mate the panel was offering",
    );
    let stopped = advance_until(
        &mut app,
        |app| app.world().get::<BleedingOut>(mate).is_none(),
        SETTLE_BUDGET,
    );
    assert!(
        stopped,
        "the sim's own stabilize must clear the carried target's bleed; it was still bleeding \
         after {SETTLE_BUDGET} frames",
    );
    assert_eq!(
        app.world().get::<LifeState>(mate),
        Some(&LifeState::Downed),
        "stabilizing leaves the gang mate downed and alive; a command wired to the execute \
         family would have killed it instead",
    );
    Ok(())
}

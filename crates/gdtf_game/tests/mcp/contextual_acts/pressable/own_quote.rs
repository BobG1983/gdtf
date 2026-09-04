//! Each panel reads its own act's cost helper, on a tuning where every other act costs less.

use bevy::app::App;
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    acts::{
        downed::{execute_tu_cost, stabilize_tu_cost},
        enter_emplacement_tu_cost, exit_emplacement_tu_cost, open_door_tu_cost, shove_tu_cost,
        throw_grenade_tu_cost,
    },
    ganger::Tu,
    tuning::{
        CombatTuning, EnterEmplacementTu, ExecuteTu, ExitEmplacementTu, OpenDoorTu, ShoveTu,
        StabilizeTu, ThrowTu,
    },
};
use gdtf_game::qa_wire::offer::{ContextualActNet, OfferPressableNet};

use super::{
    probe::{OfferAtPool, Pool, from_tuning, offer_at_pool},
    throw::an_arcing_weapon_over_a_hovered_cell,
};
use crate::{
    contextual_acts::{
        door::door_beside_the_shooter,
        emplacement::{emplacement_beside_the_shooter, manned_emplacement_under_the_shooter},
        neighbour::{
            bleeding_mate_beside_the_shooter, downed_enemy_beside_the_shooter,
            enemy_beside_the_shooter,
        },
        scene::settle,
    },
    socket_support::{TestError, TestResult},
};

/// A scene these cases build twice, once for each pool they read the offer at.
type Scene<T> = fn() -> Result<(App, McpPort, T), TestError>;

/// Every act the seven panel systems share the two shipped tuning values between.
const THE_SEVEN: [ContextualActNet; 7] = [
    ContextualActNet::Execute,
    ContextualActNet::Stabilize,
    ContextualActNet::Shove,
    ContextualActNet::OpenDoor,
    ContextualActNet::EnterEmplacement,
    ContextualActNet::ExitEmplacement,
    ContextualActNet::ThrowGrenade,
];

/// The live tuning with `dearest` priced above every other act, and those levelled with each other.
fn dearest_of_the_seven(live: CombatTuning, dearest: ContextualActNet) -> CombatTuning {
    let priced = |act: ContextualActNet| if act == dearest { 9 } else { 3 };
    CombatTuning {
        execute_tu: ExecuteTu::new(priced(ContextualActNet::Execute)),
        stabilize_tu: StabilizeTu::new(priced(ContextualActNet::Stabilize)),
        shove_tu: ShoveTu::new(priced(ContextualActNet::Shove)),
        open_door_tu: OpenDoorTu::new(priced(ContextualActNet::OpenDoor)),
        enter_emplacement_tu: EnterEmplacementTu::new(priced(ContextualActNet::EnterEmplacement)),
        exit_emplacement_tu: ExitEmplacementTu::new(priced(ContextualActNet::ExitEmplacement)),
        throw_tu: ThrowTu::new(priced(ContextualActNet::ThrowGrenade)),
        ..live
    }
}

/// What each of the seven costs under `tuning`, read through the sim's own helpers.
fn quotes(tuning: &CombatTuning) -> [(ContextualActNet, Tu); 7] {
    [
        (ContextualActNet::Execute, execute_tu_cost(tuning)),
        (ContextualActNet::Stabilize, stabilize_tu_cost(tuning)),
        (ContextualActNet::Shove, shove_tu_cost(tuning)),
        (ContextualActNet::OpenDoor, open_door_tu_cost(tuning)),
        (
            ContextualActNet::EnterEmplacement,
            enter_emplacement_tu_cost(tuning),
        ),
        (
            ContextualActNet::ExitEmplacement,
            exit_emplacement_tu_cost(tuning),
        ),
        (
            ContextualActNet::ThrowGrenade,
            throw_grenade_tu_cost(tuning),
        ),
    ]
}

/// The same scene, handed over with `act` priced above every other act on the panel.
fn priced_above_the_rest<T>(
    act: ContextualActNet,
    scene: Scene<T>,
) -> impl FnOnce() -> Result<(App, McpPort, T), TestError> {
    move || {
        let (mut app, port, carried) = scene()?;
        let Some(live) = app.world().get_resource::<CombatTuning>().cloned() else {
            return Err(
                "a running battle must hold the tuning the panel prices its acts from".into(),
            );
        };
        app.world_mut()
            .insert_resource(dearest_of_the_seven(live, act));
        settle(&mut app);
        Ok((app, port, carried))
    }
}

/// Fail unless this act is still on offer and its button flips at its own quote.
fn assert_reads_its_own_quote<T>(
    act: ContextualActNet,
    scene: Scene<T>,
    price: fn(&CombatTuning) -> Tu,
) -> TestResult {
    let short: OfferAtPool = offer_at_pool(
        act,
        priced_above_the_rest(act, scene),
        from_tuning(price),
        Pool::ShortOfTheQuote,
    )?;
    let covered: OfferAtPool = offer_at_pool(
        act,
        priced_above_the_rest(act, scene),
        from_tuning(price),
        Pool::TheQuote,
    )?;

    assert_eq!(
        short.offer.pressable,
        OfferPressableNet::new(false),
        "{act:?} costs more here than every other act on the panel, so a pool one below its own \
         quote greys it out — a panel reading another act's helper would quote less than the sim \
         charges: {short:?}",
    );
    assert_eq!(
        covered.offer.pressable,
        OfferPressableNet::new(true),
        "{act:?} lights up at exactly its own quote: {covered:?}",
    );
    Ok(())
}

#[test]
fn the_act_under_test_is_always_the_dearest_of_the_seven() {
    for act in THE_SEVEN {
        let priced = quotes(&dearest_of_the_seven(CombatTuning::default(), act));
        let Some((_, own)) = priced.iter().find(|(named, _)| *named == act).copied() else {
            unreachable!("every act this suite prices is one of the seven: {priced:?}");
        };
        for (named, quote) in priced {
            assert!(
                named == act || *quote < *own,
                "{act:?} must cost more than {named:?} here, or a panel reading {named:?}'s \
                 helper would quote enough and the swap would go unseen: {priced:?}",
            );
        }
    }
}

#[test]
fn the_execute_panel_reads_the_execute_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::Execute,
        downed_enemy_beside_the_shooter,
        execute_tu_cost,
    )
}

#[test]
fn the_stabilize_panel_reads_the_stabilize_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::Stabilize,
        bleeding_mate_beside_the_shooter,
        stabilize_tu_cost,
    )
}

#[test]
fn the_shove_panel_reads_the_shove_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::Shove,
        enemy_beside_the_shooter,
        shove_tu_cost,
    )
}

#[test]
fn the_open_door_panel_reads_the_open_door_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::OpenDoor,
        door_beside_the_shooter,
        open_door_tu_cost,
    )
}

#[test]
fn the_enter_emplacement_panel_reads_the_enter_emplacement_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::EnterEmplacement,
        emplacement_beside_the_shooter,
        enter_emplacement_tu_cost,
    )
}

#[test]
fn the_exit_emplacement_panel_reads_the_exit_emplacement_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::ExitEmplacement,
        manned_emplacement_under_the_shooter,
        exit_emplacement_tu_cost,
    )
}

#[test]
fn the_throw_panel_reads_the_throw_quote() -> TestResult {
    assert_reads_its_own_quote(
        ContextualActNet::ThrowGrenade,
        an_arcing_weapon_over_a_hovered_cell,
        throw_grenade_tu_cost,
    )
}

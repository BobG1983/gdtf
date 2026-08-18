//! Enter and exit charge exactly what their cost functions quote.

use gdtf_battle_sim::{
    acts::{
        EnterEmplacementRequested, ExitEmplacementRequested, can_enter_emplacement,
        can_exit_emplacement, enter_emplacement_tu_cost, exit_emplacement_tu_cost,
    },
    ganger::{Direction, Position, Tu},
    terrain::emplacement::{EmplacementOccupant, EmplacementState},
    test_support::{SituationBuilder, emplacement_at},
    tuning::CombatTuning,
};

use super::harness::*;

fn tuning_of(app: &bevy::app::App) -> CombatTuning {
    app.world()
        .get_resource::<CombatTuning>()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn enter_and_exit_charge_exactly_their_quotes_and_the_predicates_agree() {
    let (mut app, seed) = battle_app(0x5543_0C05);
    let emp_cell = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .with_scatter(emplacement_at(emp_cell))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let emplacement = seated_emplacement(&mut app, emp_cell);
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let tuning = tuning_of(&app);
    let enter_quote = enter_emplacement_tu_cost(&tuning);
    let exit_quote = exit_emplacement_tu_cost(&tuning);
    assert!(
        *enter_quote > 0 && *exit_quote > 0,
        "both emplacement leaves are real positive costs",
    );

    let Some(actor_pos) = app.world().get::<Position>(actor).copied() else {
        unreachable!("the spawned ganger carries a Position");
    };
    let Some(tu_before) = tu_of(&app, actor) else {
        unreachable!("the spawned ganger carries Tu");
    };
    assert!(
        *can_enter_emplacement(
            actor_pos,
            Position::new(emp_cell),
            &EmplacementState::Vacant,
            &Tu::new(tu_before),
            &tuning,
        ),
        "an adjacent actor with an ample pool may enter a vacant emplacement",
    );
    assert!(
        !*can_enter_emplacement(
            actor_pos,
            Position::new(emp_cell),
            &EmplacementState::Vacant,
            &Tu::new(enter_quote.saturating_sub(1)),
            &tuning,
        ),
        "a pool one TU below the enter quote cannot pay for it",
    );

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "the actor entered the emplacement",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_before - *enter_quote),
        "entering charged exactly enter_emplacement_tu_cost",
    );

    let Some(after_enter) = tu_of(&app, actor) else {
        unreachable!("the occupant still carries Tu");
    };
    assert!(
        *can_exit_emplacement(
            actor,
            &EmplacementState::Occupied,
            &EmplacementOccupant::new(actor),
            &Tu::new(after_enter),
            &tuning,
        ),
        "the seated occupant with an ample pool may leave",
    );
    assert!(
        !*can_exit_emplacement(
            actor,
            &EmplacementState::Occupied,
            &EmplacementOccupant::new(actor),
            &Tu::new(exit_quote.saturating_sub(1)),
            &tuning,
        ),
        "a pool one TU below the exit quote cannot pay for it",
    );

    app.world_mut()
        .write_message(ExitEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the actor left the emplacement",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(after_enter - *exit_quote),
        "leaving charged exactly exit_emplacement_tu_cost",
    );
}

//! Which reason `try_fire_request` picks: its own two, and the guard's answer it forwards.

use gdtf_battle_input::ShotRefusal;

use super::harness::{FireCase, off_grid, on_grid};

#[test]
fn an_entity_holding_no_shooter_parts_answers_the_not_a_shooter_reason() {
    let mut case = FireCase::not_a_shooter();

    assert_eq!(
        case.request(on_grid()),
        Some(Err(ShotRefusal::NotAShooter)),
        "an entity carrying none of the parts a shooter needs is turned away for that, before \
         any weapon or guard rule is read"
    );
}

#[test]
fn a_shooter_wielding_nothing_answers_the_no_firing_weapon_reason() {
    let mut case = FireCase::unarmed_shooter();

    assert_eq!(
        case.request(on_grid()),
        Some(Err(ShotRefusal::NoFiringWeapon)),
        "a shooter carrying every firing part but wielding no weapon has nothing to fire with, \
         and that is the reason it gets back"
    );
}

#[test]
fn an_empty_magazine_comes_back_as_the_guards_own_reason() {
    let mut case = FireCase::armed_shooter(0);

    assert_eq!(
        case.request(on_grid()),
        Some(Err(ShotRefusal::MagazineEmpty)),
        "an alive, paid-up shooter aiming on the grid with both hands breaks only the magazine \
         rule, so the guard answers MagazineEmpty and try_fire_request hands it back unchanged"
    );
}

#[test]
fn a_target_off_the_grid_comes_back_as_the_guards_own_reason() {
    let mut case = FireCase::armed_shooter(10);

    assert_eq!(
        case.request(off_grid()),
        Some(Err(ShotRefusal::OutOfBounds)),
        "an alive, paid-up shooter holding rounds with both hands breaks only the bounds rule, \
         so the guard answers OutOfBounds and try_fire_request hands it back unchanged"
    );
}

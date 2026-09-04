//! The enemy half of the lit area, and its refusal off the battle screen.

use gdtf_game::qa_wire::token::GangerToken;

use super::body::{LitArea, lit_area, visible_with_an_enemy};
use crate::{
    battle_setup::Standing,
    command_exchange::{BATTLE_VISIBLE, assert_refused_off_the_battle_screen},
    socket_support::{TestError, TestResult, game_app_listening},
};

#[test]
fn an_enemy_in_the_lit_area_is_listed_where_it_stands() -> TestResult {
    let (visible, enemy) = visible_with_an_enemy(Standing::Lit)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(entry) = visible.enemies.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "an enemy standing on a lit cell is in the lit area: {enemy:?} missing from \
             {visible:?}"
        );
    };
    assert_eq!(
        entry.at, enemy.at,
        "the entry names the cell the enemy actually stands on: {entry:?}",
    );
    Ok(())
}

#[test]
fn an_enemy_the_fog_hides_is_left_out_of_the_lit_area() -> TestResult {
    let (visible, enemy) = visible_with_an_enemy(Standing::Hidden)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    assert!(
        !visible.enemies.iter().any(|entry| entry.token == token),
        "an enemy the fog hides is not in the lit area: {enemy:?} appears in {visible:?}",
    );
    Ok(())
}

#[test]
fn the_lit_area_lists_enemies_the_squad_can_see_and_nothing_else() -> TestResult {
    let LitArea {
        visible,
        roster,
        turn,
        enemy: expected,
    } = lit_area()?;

    let token = GangerToken::new(expected.entity.to_bits());
    assert!(
        visible.enemies.iter().any(|enemy| enemy.token == token),
        "the battle held a squad-visible enemy, so this cross-check has something to compare: \
         {expected:?} missing from {visible:?}",
    );
    let Some(player) = turn.player else {
        unreachable!("a running battle names the faction the player commands: {turn:?}");
    };
    assert!(
        roster.gangers.iter().any(|card| card.faction == player),
        "the roster holds the player's own squad, so this case can tell one from the other: \
         {roster:?}",
    );
    for enemy in &visible.enemies {
        let card = roster
            .gangers
            .iter()
            .find(|card| card.token == enemy.token)
            .ok_or_else(|| -> TestError {
                format!(
                    "an enemy in the lit area is a roster card too — both read the same shown \
                     fog: {enemy:?} against {roster:?}"
                )
                .into()
            })?;
        assert_ne!(
            card.faction, player,
            "the lit-area list is enemies only, so the player's own gangers never reach it: \
             {card:?}",
        );
    }
    Ok(())
}

#[test]
fn the_enemy_list_comes_back_ordered() -> TestResult {
    let visible = lit_area()?.visible;

    let enemies: Vec<u64> = visible.enemies.iter().map(|entry| *entry.token).collect();
    assert!(
        !enemies.is_empty(),
        "the fixture lights an enemy, so this case has a list to check",
    );
    assert!(
        enemies.is_sorted(),
        "the enemy list is sorted before it goes out: {enemies:?}",
    );
    Ok(())
}

#[test]
fn the_lit_area_read_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_VISIBLE, "()")?;
    Ok(())
}

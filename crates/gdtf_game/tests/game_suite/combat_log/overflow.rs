use bevy::prelude::*;
use gdtf_battle_sim::acts::MoveCompleted;
use gdtf_game::test_support::CombatLogLine;

use super::harness::*;

const DEFAULT_MAX_VISIBLE: usize = 6;

const DEFAULT_MAX_VISIBLE_I32: i32 = 6;

#[test]
fn overflow_fifo_despawns_the_oldest_lines() {
    let mut app = battle_running_app();
    let at = a_lit_cell(&app);

    let total: i32 = DEFAULT_MAX_VISIBLE_I32 + 3;
    let names: Vec<String> = (0..total).map(|i| format!("Vex{i}")).collect();
    for name in &names {
        let ganger = spawn_named(&mut app, name);
        play(&mut app, MoveCompleted::new(ganger, at));
    }
    app.update();
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        DEFAULT_MAX_VISIBLE,
        "the visible line count is capped at max_visible_lines ({DEFAULT_MAX_VISIBLE}), got {} \
         lines: {texts:?}",
        texts.len(),
    );
    let survivors_start = usize::try_from(total).unwrap_or(0) - DEFAULT_MAX_VISIBLE;
    for name in names.iter().skip(survivors_start) {
        assert!(
            texts.iter().any(|line| line.starts_with(name)),
            "the newest line for {name:?} must survive the FIFO trim, got {texts:?}",
        );
    }
    let Some(oldest) = names.first() else {
        unreachable!("the case plays {total} moves, so it named that many gangers");
    };
    assert!(
        !texts.iter().any(|line| line.starts_with(oldest)),
        "the OLDEST line, for {oldest:?}, must have been FIFO-despawned, got {texts:?}",
    );
}

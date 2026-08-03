use bevy::prelude::*;
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_sim::{acts::MovementOccurred, prelude::Cell};

use super::harness::*;

const DEFAULT_MAX_VISIBLE: usize = 6;

const DEFAULT_MAX_VISIBLE_I32: i32 = 6;


#[test]
fn overflow_fifo_despawns_the_oldest_lines() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    let total: i32 = DEFAULT_MAX_VISIBLE_I32 + 3;
    for y in 0..total {
        play(
            &mut app,
            MovementOccurred::new(ganger, Cell::new(0, 0), Cell::new(0, y)),
        );
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
    let survivors_start = total - DEFAULT_MAX_VISIBLE_I32;
    for i in survivors_start..total {
        let expected = format!("Vex moved (0, 0) -> (0, {i})");
        assert!(
            texts.contains(&expected),
            "the newest line {expected:?} must survive the FIFO trim, got {texts:?}",
        );
    }
    let oldest = "Vex moved (0, 0) -> (0, 0)".to_owned();
    assert!(
        !texts.contains(&oldest),
        "the OLDEST line {oldest:?} must have been FIFO-despawned, got {texts:?}",
    );
}

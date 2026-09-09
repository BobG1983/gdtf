//! Every fx harness helper that steps time by hand puts the pinned delta back before it returns.

use std::time::Duration;

use gdtf_battle_sim::{
    falls::{FallOccurred, StoreysFallen},
    prelude::{BattleInProgress, Cell, CellLevel, Level, Position},
};

use super::harness::*;
use crate::pinned_delta::assert_reads_a_pinned_delta;

const STEP: Duration = Duration::from_millis(30);
const CELL: Cell = Cell::new(4, 4);
const LEVEL: Level = Level::new(0);
const FELL_FROM: Level = Level::new(2);

#[test]
fn step_app_restores_the_pinned_delta() {
    let mut app = headless_renderer_app();

    step_app(&mut app, STEP, 1);

    assert_reads_a_pinned_delta(&mut app);
}

#[test]
fn step_counting_impacts_restores_the_pinned_delta() {
    let mut app = headless_renderer_app();

    let counted = step_counting_impacts(&mut app, STEP, 1);

    assert_eq!(
        counted, 0,
        "no shot was played, so the step must count no resolved impact",
    );
    assert_reads_a_pinned_delta(&mut app);
}

#[test]
fn advance_past_ttl_restores_the_pinned_delta() {
    let mut app = headless_renderer_app();

    advance_past_ttl(&mut app);

    assert_reads_a_pinned_delta(&mut app);
}

#[test]
fn fire_with_zero_delta_restores_the_pinned_delta() {
    let mut app = headless_renderer_app();

    fire_with_zero_delta(&mut app);

    assert_reads_a_pinned_delta(&mut app);
}

#[test]
fn step_until_pop_restores_the_pinned_delta() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    let faller = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(CELL, LEVEL)))
        .id();
    play(
        &mut app,
        FallOccurred::new(faller, FELL_FROM, LEVEL, StoreysFallen::new(2)),
    );

    let pops = step_until_pop(&mut app, "Fell", STEP);

    assert!(
        pops.iter().any(|(text, _)| text == "Fell"),
        "the fall must pop \"Fell\" before the restored delta is read back: {pops:?}",
    );
    assert_reads_a_pinned_delta(&mut app);
}

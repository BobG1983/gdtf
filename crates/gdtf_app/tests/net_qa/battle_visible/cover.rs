//! The cover half of the lit area, checked against the inspect panel's own block.

use gdtf_app::qa_wire::inspect::{CoverBlockNet, TerrainKindNet};
use gdtf_qa_protocol::command::RunOptions;

use super::body::{InspectBody, VisibleBody, decode, lit_area};
use crate::{
    battle_reads::cell_argument,
    battle_setup::{
        battle_with_a_lit_emplacement, battle_with_a_lit_unledgered_wall, battle_with_lit_cover,
        battle_with_remembered_cover,
    },
    command_exchange::{BATTLE_INSPECT, BATTLE_VISIBLE, exchange_expected, run},
    socket_support::TestResult,
};

fn block_is_drawable(block: &CoverBlockNet) -> bool {
    *block.hp_max > 0 && *block.hp <= *block.hp_max
}

#[test]
fn cover_in_the_lit_area_is_listed_as_the_inspect_panel_would_draw_it() -> TestResult {
    let (replies, cover) = exchange_expected(battle_with_lit_cover, |cover| {
        vec![
            run(BATTLE_VISIBLE, "()", RunOptions::default()),
            run(
                BATTLE_INSPECT,
                &cell_argument(cover.at),
                RunOptions::default(),
            ),
        ]
    })?;
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.next())?;

    let Some(entry) = visible.cover.iter().find(|entry| entry.at == cover.at) else {
        unreachable!(
            "a wall or cover cell the squad can see is in the lit area's cover list: \
             {cover:?} missing from {visible:?}"
        );
    };
    let Some(block) = inspect.shown.terrain.as_ref().and_then(|half| half.cover) else {
        unreachable!(
            "the inspect panel draws a cover block for that same cell: {cover:?} gave \
             {inspect:?}"
        );
    };
    assert_eq!(
        entry.cover, block,
        "both reads describe the cell with one shared decision, so the blocks match \
         exactly: {entry:?} against {block:?}",
    );
    assert!(
        block_is_drawable(&block),
        "the block carries the cell's real cover values: {block:?}",
    );
    Ok(())
}

#[test]
fn an_emplacement_in_the_lit_area_is_listed_with_the_block_inspect_draws() -> TestResult {
    let (replies, seat) = exchange_expected(battle_with_a_lit_emplacement, |seat| {
        vec![
            run(BATTLE_VISIBLE, "()", RunOptions::default()),
            run(
                BATTLE_INSPECT,
                &cell_argument(seat.at),
                RunOptions::default(),
            ),
        ]
    })?;
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.next())?;

    let Some(terrain) = inspect.shown.terrain.as_ref() else {
        unreachable!("the seat is lit, so inspect reports its terrain half: {inspect:?}");
    };
    assert_eq!(
        terrain.kind,
        TerrainKindNet::Emplacement,
        "the authored seat reads as an emplacement: {inspect:?}",
    );
    let Some(entry) = visible.cover.iter().find(|entry| entry.at == seat.at) else {
        unreachable!(
            "an emplacement carries a real ledger entry, so the lit area lists it as \
             inspect draws it: {seat:?} missing from {visible:?}"
        );
    };
    assert_eq!(
        Some(entry.cover),
        terrain.cover,
        "both reads describe the seat with one shared decision: {entry:?} against {inspect:?}",
    );
    Ok(())
}

#[test]
fn terrain_the_ledger_has_no_entry_for_is_left_out_of_the_lit_area() -> TestResult {
    let (replies, wall) = exchange_expected(battle_with_a_lit_unledgered_wall, |wall| {
        vec![
            run(BATTLE_VISIBLE, "()", RunOptions::default()),
            run(
                BATTLE_INSPECT,
                &cell_argument(wall.at),
                RunOptions::default(),
            ),
        ]
    })?;
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.next())?;

    let Some(terrain) = inspect.shown.terrain.as_ref() else {
        unreachable!("the wall is lit, so inspect reports its terrain half: {inspect:?}");
    };
    assert_eq!(
        (terrain.kind, terrain.cover),
        (TerrainKindNet::Wall, None),
        "no ledger entry means inspect names the kind and mints no stats: {inspect:?}",
    );
    assert!(
        !visible.cover.iter().any(|entry| entry.at == wall.at),
        "every entry in the cover list carries a drawable block, so a cell with no cover \
         stats is left out: {wall:?} appears in {visible:?}",
    );
    Ok(())
}

#[test]
fn cover_the_fog_remembers_but_no_longer_lights_is_left_out_of_the_lit_area() -> TestResult {
    let (replies, cover) = exchange_expected(battle_with_remembered_cover, |_cover| {
        vec![run(BATTLE_VISIBLE, "()", RunOptions::default())]
    })?;
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.into_iter().next())?;

    assert!(
        !visible.cover.iter().any(|entry| entry.at == cover.at),
        "the cover list is the lit area only, so a cell the fog remembers but no longer lights \
         never reaches the wire: {cover:?} appears in {visible:?}",
    );
    Ok(())
}

#[test]
fn cover_the_fog_remembers_but_no_longer_lights_reports_nothing_to_inspect() -> TestResult {
    let (replies, cover) = exchange_expected(battle_with_remembered_cover, |cover| {
        vec![run(
            BATTLE_INSPECT,
            &cell_argument(cover.at),
            RunOptions::default(),
        )]
    })?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.into_iter().next())?;

    assert_eq!(
        inspect.shown.terrain, None,
        "a battle read reports only what the player can see, so a cell the screen no longer \
         lights reports nothing about the cover standing on it: {cover:?} gave {inspect:?}",
    );
    Ok(())
}

#[test]
fn every_cover_entry_carries_a_drawable_block() -> TestResult {
    let visible = lit_area()?.visible;

    for entry in &visible.cover {
        assert!(
            block_is_drawable(&entry.cover),
            "cover the panel would draw always has a full-health value: {entry:?}",
        );
    }
    Ok(())
}

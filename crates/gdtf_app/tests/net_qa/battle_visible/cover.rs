//! The cover half of the lit area, checked against the inspect panel's own block.

use gdtf_app::qa_wire::inspect::{CoverBlockNet, InspectShownNet};
use gdtf_qa_protocol::command::RunOptions;

use super::body::{InspectBody, VisibleBody, decode, lit_area};
use crate::{
    battle_reads::cell_argument,
    battle_setup::{battle_with_lit_cover, battle_with_remembered_cover},
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
    let InspectShownNet::Cover(block) = inspect.shown else {
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

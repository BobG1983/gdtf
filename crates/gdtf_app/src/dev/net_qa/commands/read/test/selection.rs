//! Which inspect-target cell lands in `hovered` and which in `pinned`.

use bevy::ecs::entity::Entity;
use gdtf_battle_input::InspectTarget;
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

use crate::dev::net_qa::{
    commands::read::battle_selection::selection_reply,
    wire::{cell::CellLevelNet, token::GangerToken},
};

fn a_cell(x: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, 4), Level::new(0))
}

#[test]
fn a_hovered_cell_is_reported_as_hovered_and_leaves_the_pin_empty() {
    let hovered = a_cell(7);
    let reply = selection_reply(None, None, Some(&InspectTarget::new(Some(hovered))));

    assert_eq!(
        reply.hovered,
        Some(CellLevelNet::from_sim(hovered)),
        "the cell the pointer is over is the reply's hovered cell: {reply:?}",
    );
    assert_eq!(reply.pinned, None, "hovering is not pinning: {reply:?}");
}

#[test]
fn a_pinned_cell_is_reported_as_pinned_beside_the_hover_it_overrides() {
    let hovered = a_cell(7);
    let pinned = a_cell(9);
    let mut target = InspectTarget::new(Some(hovered));
    target.set_pinned(pinned);
    let reply = selection_reply(None, None, Some(&target));

    assert_eq!(
        (reply.hovered, reply.pinned),
        (
            Some(CellLevelNet::from_sim(hovered)),
            Some(CellLevelNet::from_sim(pinned)),
        ),
        "the panels let the pin win, but the reply reports both so a caller can see which \
         is which: {reply:?}",
    );
}

#[test]
fn a_battle_with_nothing_hovered_or_pinned_reports_neither() {
    let reply = selection_reply(None, None, Some(&InspectTarget::default()));

    assert_eq!(
        (reply.shooter, reply.fire_mode, reply.hovered, reply.pinned),
        (None, None, None, None),
        "an untouched inspect target and no selection answer with empty fields, not \
         invented ones: {reply:?}",
    );
}

#[test]
fn the_shooter_field_names_the_selected_entity() {
    let shooter = Entity::from_raw_u32(3).unwrap_or(Entity::PLACEHOLDER);
    let reply = selection_reply(
        Some(&gdtf_battle_input::SelectedShooter::new(shooter)),
        None,
        None,
    );

    assert_eq!(
        reply.shooter,
        Some(GangerToken::new(shooter.to_bits())),
        "the reply tokenises the selected shooter, not some other entity: {reply:?}",
    );
}

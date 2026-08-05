//! Apply a left-click outcome to selection and pending intents.

use bevy::prelude::*;

use super::{LeftClickOutcome, state::PointerSelection};
use crate::{
    ActIntent, PendingActIntent,
    selection::{
        path_preview::PathPreviewTarget,
        resources::{SelectedShooter, set_selection},
    },
};

/// Mutate selection, pending intents, and path preview from a left-click outcome.
pub fn apply_left_click(
    outcome: LeftClickOutcome,
    selection: &mut PointerSelection,
    pending: &mut ResMut<PendingActIntent>,
) {
    let PointerSelection {
        selected,
        move_target: target,
        ..
    } = selection;
    match outcome {
        LeftClickOutcome::Fire(request) => {
            pending.push(ActIntent::Fire(request));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        LeftClickOutcome::Select(entity) => {
            set_selection(selected, SelectedShooter::new(entity));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        LeftClickOutcome::SetMoveTarget(cell) => {
            set_move_target(target, PathPreviewTarget::new(cell));
        }
        LeftClickOutcome::Move(request) => {
            pending.push(ActIntent::Move(request));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        LeftClickOutcome::NoOp => {}
        LeftClickOutcome::Clear => {
            set_selection(selected, SelectedShooter::cleared());
            set_move_target(target, PathPreviewTarget::cleared());
        }
    }
}

fn set_move_target(target: &mut ResMut<PathPreviewTarget>, next: PathPreviewTarget) {
    if **target != next {
        **target = next;
    }
}

//! The shared left-click COMMIT step both devices run (the GTW-259 apply half).

use bevy::prelude::*;

use super::LeftClickOutcome;
use crate::{
    ActIntent, PendingActIntent,
    selection::{
        path_preview::PathPreviewTarget,
        resources::{SelectedShooter, set_selection},
    },
};

/// Commits a [`decide_left_click`](super::decide_left_click) [`LeftClickOutcome`] to the input-layer state — the SHARED
/// apply step (GTW-259) the mouse and gamepad both run.
///
/// FIRE / MOVE push the carried act onto [`PendingActIntent`] (the ONE
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits it); SELECT sets
/// [`SelectedShooter::new`]; CLEAR sets [`SelectedShooter::cleared`]. The selection writes go
/// through `set_selection` (change-detection hygiene).
///
/// GTW-356 two-click move target:
///
/// - [`SetMoveTarget`](LeftClickOutcome::SetMoveTarget) (click-1 / re-target) writes
///   [`PathPreviewTarget::new`] (the preview shows) and dispatches NOTHING.
/// - [`Move`](LeftClickOutcome::Move) (click-2 commit) pushes [`ActIntent::Move`] AND CLEARS
///   [`PathPreviewTarget`] (the preview is consumed).
/// - [`Select`](LeftClickOutcome::Select) / [`Clear`](LeftClickOutcome::Clear) ALSO CLEAR the
///   target (C4: a new/dropped selection drops any stale preview).
/// - [`Fire`](LeftClickOutcome::Fire) clears the target too (a fire edge abandons a pending
///   move plan), and [`NoOp`](LeftClickOutcome::NoOp) (GTW-287 enemy / GTW-288 no-hover /
///   GTW-356 link-tile) does NOTHING at all — no push, no selection write, no target write —
///   so a pending target survives a no-op click.
///
/// Each write is guarded so it trips change-detection only on a real change ([`set_selection`]
/// for the selection; the `!=` guard for the target), keeping `Changed<PathPreviewTarget>` /
/// `Changed<SelectedShooter>` honest for the populate / fire-mode-sync systems.
///
/// Param-only (`bevy-traps.md` #7): the [`ResMut<SelectedShooter>`] /
/// [`ResMut<PendingActIntent>`] / [`ResMut<PathPreviewTarget>`] writes, no `&mut World`.
pub fn apply_left_click(
    outcome: LeftClickOutcome,
    selected: &mut ResMut<SelectedShooter>,
    pending: &mut ResMut<PendingActIntent>,
    target: &mut ResMut<PathPreviewTarget>,
) {
    match outcome {
        // A fire edge abandons any pending move target (C4-adjacent: a different act took over).
        LeftClickOutcome::Fire(request) => {
            pending.push(ActIntent::Fire(request));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        // A new selection drops the previous selection's stale preview (C4).
        LeftClickOutcome::Select(entity) => {
            set_selection(selected, SelectedShooter::new(entity));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        // Click-1 / re-target: set the preview target, dispatch nothing.
        LeftClickOutcome::SetMoveTarget(cell) => {
            set_move_target(target, PathPreviewTarget::new(cell));
        }
        // Click-2 commit: dispatch the move AND consume the preview target.
        LeftClickOutcome::Move(request) => {
            pending.push(ActIntent::Move(request));
            set_move_target(target, PathPreviewTarget::cleared());
        }
        // GTW-287 enemy / GTW-288 no-hover / GTW-356 link-tile: do nothing, leave the selection
        // AND the pending target exactly as they were.
        LeftClickOutcome::NoOp => {}
        // CLEAR drops the selection AND any pending preview (C4: deselecting clears the target).
        LeftClickOutcome::Clear => {
            set_selection(selected, SelectedShooter::cleared());
            set_move_target(target, PathPreviewTarget::cleared());
        }
    }
}

/// Writes `next` into `target` only on a real change (change-detection hygiene), so a no-op
/// clear / set does not spuriously trip `Changed<PathPreviewTarget>` (which
/// [`populate_path_preview`](crate::populate_path_preview) keys off via its own `!=` guard).
fn set_move_target(target: &mut ResMut<PathPreviewTarget>, next: PathPreviewTarget) {
    if **target != next {
        **target = next;
    }
}

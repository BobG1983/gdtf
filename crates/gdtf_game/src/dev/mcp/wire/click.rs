//! What a left click decided, and what the act it started answered.

use gdtf_battle_input::LeftClickOutcome;
use serde::{Deserialize, Serialize};

use super::act::ActReply;

/// Which of the six things the game's left-click decision did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClickDecisionNet {
    /// Fire at the clicked cell.
    Fire,
    /// Select the player ganger standing there.
    Select,
    /// Pin the cell as a move target.
    SetMoveTarget,
    /// Confirm the move to the already-pinned cell.
    Move,
    /// Do nothing.
    NoOp,
    /// Clear the selection and the pinned target.
    Clear,
}

impl ClickDecisionNet {
    /// Mirror the game's own decision.
    #[must_use]
    pub const fn from_game(outcome: &LeftClickOutcome) -> Self {
        match outcome {
            LeftClickOutcome::Fire(_) => Self::Fire,
            LeftClickOutcome::Select(_) => Self::Select,
            LeftClickOutcome::SetMoveTarget(_) => Self::SetMoveTarget,
            LeftClickOutcome::Move(_) => Self::Move,
            LeftClickOutcome::NoOp => Self::NoOp,
            LeftClickOutcome::Clear => Self::Clear,
        }
    }
}

/// What `input.click_cell` answers: the decision, and the act it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClickReply {
    /// What the game's left-click decision did with the cell.
    pub decision: ClickDecisionNet,
    /// The act-log window or typed refusal the decision produced, absent when it pushed no act.
    pub act:      Option<ActReply>,
}

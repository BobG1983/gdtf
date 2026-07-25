//! [`UiStackView`] — which UI stack the DEV swap harness is rendering, and which pair it
//! is comparing (GTW-816).
//!
//! Reported on [`BattleView::ui_stack`](super::BattleView::ui_stack) so an agent that
//! swaps stacks can read back which one is live and assert behavioural parity across the
//! swap. The stack NAME itself is [`UiStackNet`], owned by the intent family (it is a
//! [`SwapUiStack`](crate::intent::NetIntent::SwapUiStack) payload first) — this module
//! adds only the read-model shapes around it, and does not re-export it.

use serde::{Deserialize, Serialize};

use crate::intent::UiStackNet;

/// The two stacks currently under comparison, in the harness's own order.
///
/// The pair is reported (not just the live one) because which two stacks are being
/// compared is a fact of the running build, not a constant: a comparison surface names its
/// own baseline and candidate, and an agent asserting parity needs to know what it is
/// toggling between. Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiStackPairView {
    /// The stack the comparison starts from.
    pub baseline:  UiStackNet,
    /// The stack the comparison is judging against the baseline.
    pub candidate: UiStackNet,
}

impl UiStackPairView {
    /// Build a comparison pair from its baseline and candidate.
    #[must_use]
    pub const fn new(baseline: UiStackNet, candidate: UiStackNet) -> Self {
        Self {
            baseline,
            candidate,
        }
    }
}

/// The DEV UI-stack swap harness's state — the pair under comparison plus the live stack.
///
/// Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiStackView {
    /// The two stacks under comparison.
    pub comparison: UiStackPairView,
    /// Which of the pair is rendering right now.
    pub live:       UiStackNet,
}

impl UiStackView {
    /// Build a harness view from the compared pair and the live stack.
    #[must_use]
    pub const fn new(comparison: UiStackPairView, live: UiStackNet) -> Self {
        Self { comparison, live }
    }
}

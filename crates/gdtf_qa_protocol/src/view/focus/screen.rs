//! [`FocusView`] — the whole screen's focus-navigable control snapshot (GTW-802).

use serde::{Deserialize, Serialize};

use crate::{ids::FocusTargetNet, view::focus::focusable::FocusableView};

/// The current screen's **focus snapshot** — which control holds focus, and every control
/// focus can reach.
///
/// Rides in [`AppFlowView::focus`](crate::view::AppFlowView::focus) as
/// `Option<FocusView>` — `None` when the app is on a screen with no focus graph at all,
/// `Some` with the focused control + the full focusable list otherwise.
///
/// [`focused`](Self::focused) duplicates the per-row
/// [`focused`](FocusableView::focused) flag deliberately: it answers "activate what?" in one
/// read, without scanning the list. It is `None` when the screen has focusables but nothing
/// currently holds focus. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FocusView {
    /// The control that currently holds input focus, if any.
    pub focused:    Option<FocusTargetNet>,
    /// Every control focus can reach on this screen, in a stable token order.
    pub focusables: Vec<FocusableView>,
}

impl FocusView {
    /// Build a focus snapshot from the focused control (if any) and the focusable list.
    #[must_use]
    pub const fn new(focused: Option<FocusTargetNet>, focusables: Vec<FocusableView>) -> Self {
        Self {
            focused,
            focusables,
        }
    }
}

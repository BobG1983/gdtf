//! [`FocusableKindNet`] — what KIND of control a focusable row describes (GTW-802).

use serde::{Deserialize, Serialize};

/// The **kind** of a focusable control — what a client is about to activate.
///
/// A closed, independent serde enum (never a leak of a Bevy widget type): a future control
/// kind forces a new variant rather than silently reading as a button. It tells a client
/// what activating the row will DO — a [`Button`](Self::Button) commits an action, a
/// [`Checkbox`](Self::Checkbox) flips its
/// [`checked`](crate::view::FocusableView::checked) state — so an agent can decide whether
/// it needs to read the value back afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusableKindNet {
    /// A push button — activating it commits the button's action.
    Button,
    /// A checkbox / toggle — activating it flips its checked state.
    Checkbox,
    /// A focusable control of no kind this vocabulary names yet.
    Other,
}

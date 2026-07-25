//! The focus-enumeration scalar newtypes — a focusable control's caption, its enabled
//! flag, a checkbox's checked state, and whether it currently holds focus (GTW-802).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A focusable control's **on-screen label** — its caption (e.g. `"Continue"`, `"Sound"`).
///
/// So a client can name a control by its purpose rather than an opaque token when it
/// chooses which one to [`FocusControl`](crate::envelope::QaRequest::FocusControl). Its own
/// type rather than a re-use of
/// [`MenuItemLabelNet`](crate::view::MenuItemLabelNet): a focusable control is a different
/// family from a menu item, and a family never presents a sibling's type as its own API
/// (module-layout rule 7). A named newtype over the caption string (no-bare-types),
/// serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusableLabelNet(String);

impl FocusableLabelNet {
    /// Build a focusable label from its on-screen caption.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// Whether a focusable control is **currently activatable** — the wire mirror of "the
/// control is not disabled".
///
/// A disabled control is still LISTED (so the client sees the whole focus chain) but marked
/// `enabled == false`. A private-inner newtype over `bool` (no-bare-types),
/// serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusableEnabledNet(bool);

impl FocusableEnabledNet {
    /// Build an enabled flag — `true` when the control can be activated right now.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }
}

/// A **checkbox's current on/off state** — carried only by a
/// [`Checkbox`](crate::view::FocusableKindNet::Checkbox) row.
///
/// So an agent can read a toggle's value before AND after flipping it, and confirm the flip
/// landed from a follow-up `GetAppFlow` alone — without a screenshot. A private-inner
/// newtype over `bool` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusableCheckedNet(bool);

impl FocusableCheckedNet {
    /// Build a checked state — `true` when the checkbox is currently checked.
    #[must_use]
    pub const fn new(checked: bool) -> Self {
        Self(checked)
    }
}

/// Whether this control **currently holds input focus**.
///
/// Redundant with [`FocusView::focused`](crate::view::FocusView::focused) on purpose: the
/// per-row flag lets a client read the whole chain and see the cursor in one pass, while the
/// screen-level field answers "activate what?" in one read. A private-inner newtype over
/// `bool` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusedNet(bool);

impl FocusedNet {
    /// Build a focused flag — `true` when the control currently holds input focus.
    #[must_use]
    pub const fn new(focused: bool) -> Self {
        Self(focused)
    }
}

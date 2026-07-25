//! [`FocusableView`] — one enumerated focus-navigable control (GTW-802).

use serde::{Deserialize, Serialize};

use crate::{
    ids::FocusTargetNet,
    view::focus::{
        kind::FocusableKindNet,
        label::{FocusableCheckedNet, FocusableEnabledNet, FocusableLabelNet, FocusedNet},
    },
};

/// One enumerable focus-navigable **control** — its focus token, label, kind, enabled flag,
/// a checkbox's current value, and whether it holds focus right now.
///
/// The [`token`](Self::token) is a [`FocusTargetNet`] a client echoes straight back to
/// [`FocusControl`](crate::envelope::QaRequest::FocusControl) to point focus at, or
/// activate, this control through the game's real focus/interaction path. Serde default
/// shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FocusableView {
    /// The control's focus handle — echoed back by
    /// [`FocusControl`](crate::envelope::QaRequest::FocusControl).
    pub token:   FocusTargetNet,
    /// The control's on-screen caption.
    pub label:   FocusableLabelNet,
    /// What kind of control this is — what activating it will do.
    pub kind:    FocusableKindNet,
    /// Whether the control can be activated right now (a disabled control is listed but
    /// `false`).
    pub enabled: FocusableEnabledNet,
    /// A [`Checkbox`](FocusableKindNet::Checkbox)'s current on/off state; `None` for every
    /// other kind, which has no value to read.
    pub checked: Option<FocusableCheckedNet>,
    /// Whether this control currently holds input focus.
    pub focused: FocusedNet,
}

impl FocusableView {
    /// Build a focusable-control view from its token, label, kind, enabled flag, optional
    /// checked state, and focused flag.
    #[must_use]
    pub const fn new(
        token: FocusTargetNet,
        label: FocusableLabelNet,
        kind: FocusableKindNet,
        enabled: FocusableEnabledNet,
        checked: Option<FocusableCheckedNet>,
        focused: FocusedNet,
    ) -> Self {
        Self {
            token,
            label,
            kind,
            enabled,
            checked,
            focused,
        }
    }
}

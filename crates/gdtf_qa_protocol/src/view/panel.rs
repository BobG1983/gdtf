//! [`PanelButtonView`] — the battlescape HUD focus-button token handout (GTW-789).
//!
//! The wire mirror of the focus-navigable HUD buttons a QA client needs: every
//! battlescape panel button that keyboard focus can land on (the action bar, the weapon
//! panel's Reload, the contextual act cluster), with its focus TOKEN, its Tab-chain
//! ordinal, and its human-readable label. Without this handout
//! [`NetIntent::SetFocus`](crate::intent::NetIntent::SetFocus) would be dead wire surface
//! against a panel button — a client could only focus a token a `view` handed it, and the
//! only tokens on the wire named gangers / doors / emplacements, never a HUD button
//! (GTW-782's focus ring was therefore un-drivable over the wire; the GTW-694 ruling — a
//! client only ever targets a token a view minted).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::FocusTargetNet;

/// A focus button's stable **Tab-chain ordinal** — the wire mirror of the game-side
/// `PanelNavOrder` (whose inner is a `u16`).
///
/// Lower orders lead the left-to-right Tab traversal (the action bar reads first, the
/// contextual cluster last). Surfaced so a client can predict which button
/// [`SetFocus`](crate::intent::NetIntent::SetFocus) + a Tab step lands on. A private-inner
/// newtype (no-bare-types), serde-transparent so it rides the wire as its bare `u16`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelNavOrderNet(u16);

impl PanelNavOrderNet {
    /// Build a Tab-chain ordinal from its raw order value.
    #[must_use]
    pub const fn new(order: u16) -> Self {
        Self(order)
    }
}

/// A focus button's **human-readable label** — its on-screen caption (e.g. `"End Turn"`,
/// `"Reload"`, `"Level +"`).
///
/// So a client can name a button by its purpose rather than an opaque token when it
/// chooses which one to [`SetFocus`](crate::intent::NetIntent::SetFocus) onto. A named
/// newtype over the caption string (no-bare-types), serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelButtonLabelNet(String);

impl PanelButtonLabelNet {
    /// Build a button label from its on-screen caption.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// One focus-navigable battlescape HUD **button** — its focus token, Tab-chain ordinal,
/// and label.
///
/// The [`token`](Self::token) is a [`FocusTargetNet`] a client echoes straight back to
/// [`SetFocus`](crate::intent::NetIntent::SetFocus) to point the game's UI input focus at
/// this button (making its GTW-782 focus ring appear); the [`order`](Self::order) predicts
/// its position in the Tab traversal chain; the [`label`](Self::label) names its purpose.
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelButtonView {
    /// The button's focus handle — echoed back by
    /// [`SetFocus`](crate::intent::NetIntent::SetFocus).
    pub token: FocusTargetNet,
    /// The button's stable position in the left-to-right Tab traversal chain.
    pub order: PanelNavOrderNet,
    /// The button's on-screen caption.
    pub label: PanelButtonLabelNet,
}

impl PanelButtonView {
    /// Build a panel-button view from its focus token, Tab-chain ordinal, and label.
    #[must_use]
    pub const fn new(
        token: FocusTargetNet,
        order: PanelNavOrderNet,
        label: PanelButtonLabelNet,
    ) -> Self {
        Self {
            token,
            order,
            label,
        }
    }
}

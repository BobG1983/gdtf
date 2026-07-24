//! [`MenuView`] — the generic menu enumeration handout (GTW-787).
//!
//! The wire mirror of "what menu am I on, and what can I click": the active menu's
//! [`id`](MenuView::id) and its [`items`](MenuView::items), each a curated
//! `{token, label, enabled}` entry. It rides in
//! [`AppFlowView::menu`](crate::view::AppFlowView::menu) — the lifecycle snapshot a QA
//! client polls first — so one `GetAppFlow` read answers both "where is the app" and
//! "which menu button can I activate".
//!
//! The [`token`](MenuItemView::token) is a [`FocusTargetNet`] a client echoes straight
//! back to [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem) to activate
//! the item through the game's real focus-activation path (the same message an `Enter`
//! keypress raises), mirroring the GTW-789 panel-button token handout — a client only
//! ever targets a token a view minted (the GTW-694 ruling). A new scene gets this
//! enumeration for free by tagging its menu with the game-side menu model (a
//! menu-identity marker on the scene root + a menu-item marker on each button); no wire
//! type changes per new menu.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::FocusTargetNet;

/// The active menu's **identity** — a stable, human-readable name (e.g. `"MainMenu"`).
///
/// So a QA client can tell WHICH menu it is on (the main menu vs. an options screen)
/// rather than inferring it from the item set. A named newtype over the identity string
/// (no-bare-types), serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MenuIdNet(String);

impl MenuIdNet {
    /// Build a menu identity from its stable name.
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self(id)
    }
}

/// A menu item's **on-screen label** — its caption (e.g. `"Battlescape"`, `"Quit"`).
///
/// So a client can name a menu item by its purpose rather than an opaque token when it
/// chooses which one to [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem).
/// A named newtype over the caption string (no-bare-types), serde-transparent;
/// `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MenuItemLabelNet(String);

impl MenuItemLabelNet {
    /// Build a menu-item label from its on-screen caption.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// Whether a menu item is **currently activatable** — the wire mirror of "the button is
/// not disabled".
///
/// A disabled item is still LISTED (so the client sees the full menu) but marked
/// `enabled == false`; every item marked `enabled == true` is genuinely activatable via
/// [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem). A private-inner
/// newtype over `bool` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MenuItemEnabledNet(bool);

impl MenuItemEnabledNet {
    /// Build an enabled flag — `true` when the item can be activated right now.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }
}

/// One enumerable menu **item** — its activation token, label, and enabled flag.
///
/// The [`token`](Self::token) is a [`FocusTargetNet`] a client echoes straight back to
/// [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem) to activate this
/// item through the game's real focus-activation path; the [`label`](Self::label) names
/// its purpose; [`enabled`](Self::enabled) says whether it can be activated right now.
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MenuItemView {
    /// The item's activation handle — echoed back by
    /// [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem).
    pub token:   FocusTargetNet,
    /// The item's on-screen caption.
    pub label:   MenuItemLabelNet,
    /// Whether the item can be activated right now (a disabled item is listed but
    /// `false`).
    pub enabled: MenuItemEnabledNet,
}

impl MenuItemView {
    /// Build a menu-item view from its activation token, label, and enabled flag.
    #[must_use]
    pub const fn new(
        token: FocusTargetNet,
        label: MenuItemLabelNet,
        enabled: MenuItemEnabledNet,
    ) -> Self {
        Self {
            token,
            label,
            enabled,
        }
    }
}

/// The current menu **snapshot** — the active menu's identity and its items.
///
/// Rides in [`AppFlowView::menu`](crate::view::AppFlowView::menu) as
/// `Option<MenuView>` — `None` when the app is not on an enumerable menu (mid-battle, or
/// a scene with no menu model), `Some` with the identity + full item list otherwise.
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MenuView {
    /// The active menu's identity.
    pub id:    MenuIdNet,
    /// The menu's items, in a stable declaration order.
    pub items: Vec<MenuItemView>,
}

impl MenuView {
    /// Build a menu snapshot from its identity and item list.
    #[must_use]
    pub const fn new(id: MenuIdNet, items: Vec<MenuItemView>) -> Self {
        Self { id, items }
    }
}

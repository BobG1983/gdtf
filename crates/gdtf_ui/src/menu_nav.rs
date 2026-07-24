//! The generic menu-enumeration model (GTW-787).
//!
//! A tiny, scene-agnostic marker vocabulary that lets a QA harness ask "what menu am I
//! on, what can I click?" over the `net_qa` wire and click by reference — without any
//! per-scene wiring on the wire side.
//!
//! A scene opts in by tagging its UI with two markers:
//!
//! - [`MenuScreen`] on the menu's root entity — names the active menu (its identity).
//! - [`MenuItem`] on each activatable button entity — marks it as an enumerable,
//!   activatable menu item.
//!
//! The `net_qa` snapshot builder reads these generically: the (single) [`MenuScreen`] is
//! the menu identity, and every [`MenuItem`] entity is projected into a wire item (its
//! `Entity` minted into a focus token, its caption read off its child `Text`, its enabled
//! flag read from the absence of [`DisabledButton`](crate::DisabledButton)). Activation
//! echoes the token back and raises the SAME focus-activation message an `Enter` keypress
//! raises ([`FocusActivated`](crate::focus_nav::FocusActivated)), so any scene that already
//! consumes that message gets QA activation for free.
//!
//! Both markers are state-scoped by the scene's normal `DespawnOnExit`, so no resource
//! lifecycle is needed — leaving a menu tears the markers down with the rest of the tree,
//! and the wire menu view becomes `None`.

use bevy::prelude::*;

/// The active menu's **identity** — a stable, human-readable name (e.g. `"MainMenu"`).
///
/// A named newtype over the identity string (no-bare-types), `Deref`ing to `&str` for
/// read access; the `net_qa` snapshot builder mints it into the wire `MenuIdNet`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuName(String);

impl MenuName {
    /// Build a menu name from its stable identity string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Marks a menu's **root** entity and names the menu — the identity a QA client reads as
/// "which menu am I on".
///
/// Placed on the scene's menu-root node (state-scoped via the scene's `DespawnOnExit`, so
/// it is torn down on leave). The `net_qa` snapshot builder expects at most one live
/// `MenuScreen`; its [`name`](MenuScreen::name) is the menu identity, and the presence of a
/// `MenuScreen` is what makes the wire menu view `Some` rather than `None`.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuScreen {
    /// The active menu's identity name.
    name: MenuName,
}

impl MenuScreen {
    /// Mark a menu root with its identity name.
    #[must_use]
    pub const fn new(name: MenuName) -> Self {
        Self { name }
    }

    /// The menu's identity name — the `net_qa` snapshot builder's read.
    #[must_use]
    pub const fn name(&self) -> &MenuName {
        &self.name
    }
}

/// Marks one **activatable menu button** as an enumerable menu item.
///
/// Placed on each menu button entity (state-scoped via the scene's `DespawnOnExit`). The
/// `net_qa` snapshot builder projects every live `MenuItem` entity into a wire menu item,
/// and the wire `ActivateMenuItem` request resolves a token back to a live `MenuItem` to
/// activate it. A unit marker: presence on an entity is the whole signal (no-bare-types
/// rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MenuItem;

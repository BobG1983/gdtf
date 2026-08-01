//! The generic menu-enumeration model (GTW-787).
//!
//! A tiny, scene-agnostic marker vocabulary that lets a QA client ask "what menu am I on,
//! what can I click?" and click by reference — without any per-scene wiring on the reading
//! side.
//!
//! A scene opts in by tagging its UI with two markers:
//!
//! - [`MenuScreen`] on the menu's root entity — names the active menu (its identity).
//! - [`MenuItem`] on each activatable button entity — marks it as an enumerable,
//!   activatable menu item.
//!
//! A reader walks them generically: the (single) [`MenuScreen`] is the menu identity, and
//! every [`MenuItem`] entity is one listed item (its caption read off its child `Text`, its
//! enabled flag read from the absence of [`DisabledButton`](crate::DisabledButton)).
//! Activating one raises the SAME focus-activation message an `Enter` keypress raises
//! ([`FocusActivated`](crate::focus_nav::FocusActivated)), so any scene that already
//! consumes that message gets QA activation for free.
//!
//! **Nothing reads these markers today.** GTW-943 removed the QA request that enumerated
//! them along with the rest of the pre-command wire; the menu QA command that replaces it is
//! a later ticket in the same epic. The markers stay on the UI so that command needs no
//! per-scene edit when it arrives.
//!
//! Both markers are state-scoped by the scene's normal `DespawnOnExit`, so no resource
//! lifecycle is needed — leaving a menu tears the markers down with the rest of the tree, and
//! a scene with no live [`MenuScreen`] has no menu up.

use bevy::prelude::*;

/// The active menu's **identity** — a stable, human-readable name (e.g. `"MainMenu"`).
///
/// A named newtype over the identity string (no-bare-types), `Deref`ing to `&str` for
/// read access.
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
/// it is torn down on leave). At most one `MenuScreen` is live at a time; its
/// [`name`](MenuScreen::name) is the menu identity, and no live `MenuScreen` means no menu
/// is up.
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

    /// The menu's identity name.
    #[must_use]
    pub const fn name(&self) -> &MenuName {
        &self.name
    }
}

/// Marks one **activatable menu button** as an enumerable menu item.
///
/// Placed on each menu button entity (state-scoped via the scene's `DespawnOnExit`). A
/// reader lists every live `MenuItem` entity and activates one by raising
/// [`FocusActivated`](crate::focus_nav::FocusActivated) on it. A unit marker: presence on an
/// entity is the whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MenuItem;

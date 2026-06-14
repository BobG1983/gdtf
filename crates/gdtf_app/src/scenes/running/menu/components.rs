//! Marker components for the main-menu buttons (GTW-121).
//!
//! Each menu button carries a unit-struct marker naming its role, so other
//! systems (the GTW-122 action layer, the nav-graph wiring, tests) can find a
//! specific button by its meaning rather than by spawn order or label text. The
//! markers are unit structs — presence alone is the signal (no-bare-types rule:
//! a button's identity is a named type, never a bare string compared at runtime).
//!
//! The disabled-state marker ([`DisabledButton`](gdtf_ui::DisabledButton)) is
//! owned by `gdtf_ui`; this module owns only the menu-specific *which-button*
//! identities.
//!
//! Visibility follows the crate's test-only-surface convention (GTW-145): each
//! marker is declared through [`crate::support_item!`], which widens it to `pub`
//! under the `test-support` feature — so the external integration tests can name
//! it through [`crate::test_support`] — and keeps it `pub(crate)` otherwise, so it
//! stays internal in production and satisfies `unreachable_pub`. This mirrors the
//! state enums and `ScenesPlugin`; declaring the markers as plain `pub struct`
//! (the original GTW-121 form) failed to build the `grimdark_turfwar` binary,
//! which compiles `gdtf_app` WITHOUT `test-support`.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **Battlescape** menu button — launches a mission (handler wired in
    /// GTW-122). It receives initial focus when the menu is spawned and is the first
    /// node in the vertical navigation chain.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BattlescapeButton;
}

crate::support_item! {
    /// Marks the **Options** menu button — opens the options screen (handler wired in
    /// GTW-122). It is the middle node of the vertical navigation chain.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsButton;
}

crate::support_item! {
    /// Marks the **`HiveScape`** menu button — the disabled placeholder for the future
    /// campaign / turf-war layer (`GameState::HiveScape`; see `docs/glossary.md`).
    ///
    /// It is spawned with [`DisabledButton`](gdtf_ui::DisabledButton) (dimmed, no
    /// handler) and sits directly above [`QuitButton`]. It is deliberately **omitted**
    /// from the navigation chain: 0.18.1 has no built-in "skip disabled" in
    /// directional navigation, so leaving it out of the edge slice is how it stays
    /// unreachable by focus while still visible.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HiveScapeButton;
}

crate::support_item! {
    /// Marks the **Quit** menu button — exits the game (handler wired in GTW-122). It
    /// is visually last and is the final node of the vertical navigation chain.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct QuitButton;
}

crate::support_item! {
    /// Marks the menu **title** text node ("GRIMDARK TURFWAR").
    ///
    /// Distinguishes the title from button captions so the spawn system can find it,
    /// and documents that this entity carries the larger
    /// [`Themed(ThemeRole::Title)`](gdtf_ui::themed::Themed) role rather than the
    /// plain text role buttons use.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MenuTitle;
}

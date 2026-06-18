//! Marker components for the battlescape weapon panel (GTW-275, bottom-left).
//!
//! The weapon panel is a battle-scoped `gdtf_ui` panel showing the selected player
//! ganger's weapon: a graphic PLACEHOLDER (no per-weapon art / items atlas), the
//! weapon's [`WeaponName`](gdtf_battle_sim::WeaponName), the magazine `"cur/max"` text,
//! a LIVE Reload button, and throwable placeholder slot(s). Its tree is a
//! [`spawn_panel`](gdtf_ui::spawn_panel) box (the [`WeaponPanelRoot`] marker) holding
//! those children; the per-widget markers below let the per-update mutate find each by
//! its stored id ([[ui-mutate-not-respawn]]). UI/view only — it reads the sim's weapon
//! components + the input crate's `SelectedShooter`, owns no combat rule.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the weapon-panel tree (the
    /// [`spawn_panel`](gdtf_ui::spawn_panel) box holding the weapon content), so the
    /// `OnExit(BattleRunning)` despawn finds and recursively tears down the whole panel by
    /// this one marker rather than tracking each child.
    ///
    /// Widened toward `crate::test_support` via [`support_item!`](crate::support_item) so the
    /// GTW-275 AC tests can assert the panel's presence, AND re-exported to the battlescape
    /// neighborhood so the sibling `set_world_viewport` system can MEASURE the panel root's
    /// [`ComputedNode`](bevy::ui::ComputedNode) size to inset the world-map viewport's
    /// LEFT/BOTTOM margins (AC9). A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponPanelRoot;
}

crate::support_item! {
    /// Marks the weapon panel's **content container** — the column holding the graphic /
    /// name / magazine / reload (everything EXCEPT the throwable placeholders), hidden as a
    /// unit when there is no selection / no weapon (AC9 empty state). A unit marker:
    /// presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponContent;
}

crate::support_item! {
    /// Marks the weapon panel's **name** [`Text`](bevy::prelude::Text) line — the selected
    /// weapon's [`WeaponName`](gdtf_battle_sim::WeaponName), mutated in place by the update.
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponNameText;
}

crate::support_item! {
    /// Marks the weapon panel's **magazine** [`Text`](bevy::prelude::Text) line — the
    /// `"cur/max"` round count (e.g. "30/30") from the
    /// [`Magazine`](gdtf_battle_sim::Magazine) grouping, mutated in place. Shown only when
    /// the weapon has a magazine (`size > 0`), else hidden. A unit marker: presence on an
    /// entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponMagazineText;
}

crate::support_item! {
    /// Marks the weapon panel's **LIVE Reload button** (GTW-275 — NOT a `DisabledButton`).
    /// A press pushes [`ActIntent::Reload`](gdtf_battle_input::ActIntent::Reload) onto the
    /// shared act-intent seam (the action-bar button precedent). Shown when the weapon has a
    /// magazine (`size > 0`), else [`Visibility::Hidden`] (mutated, never despawned). A unit
    /// marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ReloadButton;
}

//! Marker components for the battlescape weapon cluster (GTW-275 / GTW-295 / GTW-298,
//! bottom-left).
//!
//! The weapon cluster is a battle-scoped `gdtf_ui` panel group laid out to the AUTHORITATIVE
//! structure (GTW-298, user 2026-06-18): the [`WeaponPanelRoot`] **Overall Weapon Panel** is a
//! 2×2 grid — a LEFT column of the **Combined Weapon Panel** ([`CombinedWeaponPanel`], top 3/4
//! height) over the **Firemode Panel** (bottom 1/4 height) and a RIGHT column of the **Item
//! Panel** ([`WeaponItemPanel`], same 3/4 height) over the **Aim Panel** ([`AimPanel`], same
//! 1/4 height). A SEPARATE **Stance Panel** sits to the RIGHT of the Overall Weapon Panel
//! (same height, fixed-% width), wrapping the three stance toggles.
//!
//! The Combined Weapon Panel is ONE bordered box: a full-width [`WeaponImage`] placeholder
//! (top 1/2 height) over an info block (bottom 1/2 height) of the [`WeaponContent`] weapon-text
//! block (name + magazine, FULL width) STACKED OVER the LIVE [`ReloadButton`]'s own row (GTW-733
//! — Reload moved out of the name's row into its own row below, so a long shipped weapon name
//! and the button never occupy the same pixels). The Item
//! Panel holds two stacked DISABLED [`WeaponItemButton`]s (items are not modeled yet). The
//! Firemode / Aim / Stance panels host the controls RELOCATED from the action bar (GTW-298):
//! the firemode toggles, the aim toggle, and the stance toggles keep their action-bar markers
//! so the existing press → intent + active-mark systems drive them parent-agnostically.
//!
//! The per-widget markers below let the per-update mutate find each by its stored id
//! ([[ui-mutate-not-respawn]]); the structural markers let the AC tests assert the hierarchy.
//! UI/view only — it reads the sim's weapon components + the input crate's `SelectedShooter`,
//! owns no combat rule.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the weapon-cluster tree — the **Overall Weapon Panel** box
    /// holding the 2×2 grid, so the `OnExit(BattleRunning)` despawn finds and recursively tears
    /// down the whole cluster by this one marker rather than tracking each child.
    ///
    /// Widened toward `crate::test_support` via `support_item!` so the
    /// AC tests can assert the panel's presence + measure (it sits IN the bottom bar — the
    /// layout-overhaul viewport insets the map by the BOTTOM BAR, not this panel). A unit
    /// marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponPanelRoot;
}

crate::support_item! {
    /// Marks the weapon cluster's **content container** — the FULL-WIDTH weapon-text block
    /// inside the Combined Weapon Panel's info block holding the name / magazine lines (the
    /// [`ReloadButton`] sits in its OWN row below, a SIBLING of this block, not a child, so it
    /// stays framed in the empty state — GTW-733: Reload moved out of the name's row so the two
    /// never share pixels), hidden as a unit when there is no selection / no weapon (AC9 empty
    /// state, via [`Display::None`](bevy::ui::Display)). A unit marker: presence on an entity
    /// is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponContent;
}

crate::support_item! {
    /// Marks the **Combined Weapon Panel** (top-left grid cell, top 3/4 height) — the ONE
    /// bordered box wrapping the full-width [`WeaponImage`] over the info block of [the
    /// [`WeaponContent`] weapon-text block, stacked over the LIVE [`ReloadButton`]'s own row
    /// (GTW-733)]. A structural marker letting the AC tests assert the grid cell exists
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombinedWeaponPanel;
}

crate::support_item! {
    /// Marks the **Weapon Image** placeholder inside the Combined Weapon Panel — the full-width
    /// "no image" framed box (top 1/2 height) standing in for per-weapon art that does NOT
    /// exist (the items atlas is deliberately not loaded). A structural marker letting the AC
    /// tests assert the image cell exists (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponImage;
}

crate::support_item! {
    /// Marks the **Item Panel** (top-right grid cell, same 3/4 height as the Combined panel) —
    /// the framed box holding the two stacked DISABLED [`WeaponItemButton`]s. A structural
    /// marker letting the AC tests assert the grid cell exists (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemPanel;
}

crate::support_item! {
    /// Marks a **DISABLED Item Button** in the Item Panel — one of TWO stacked
    /// [`gdtf_ui`](gdtf_ui) buttons (1/2 height each, full width) standing in for unmodeled
    /// inventory items. Each carries [`DisabledButton`](gdtf_ui::DisabledButton) so it renders
    /// in the disabled color and emits no interaction (items are not implemented yet). A
    /// structural marker letting the AC tests assert both item buttons exist (no-bare-types
    /// rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemButton;
}

crate::support_item! {
    /// Marks the **Aim Panel** (bottom-right grid cell, same 1/4 height as the Firemode panel,
    /// same width as the Item Panel) — the framed box wrapping the relocated Aim toggle (the
    /// action-bar `AimToggleButton`, GTW-298) laid out as a ROW: an [`AimLabel`] "Aim" caption
    /// on the LEFT, the toggle [`Switch`](gdtf_ui::Switch) on the RIGHT, matching the mockup's
    /// "AIM \[switch\]" reading. A structural marker letting the AC tests assert the grid cell
    /// exists (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimPanel;
}

crate::support_item! {
    /// Marks the Aim Panel's **"Aim" caption** [`Text`](bevy::prelude::Text) — the text label
    /// laid out to the LEFT of the relocated Aim toggle so the control reads "Aim \[switch\]"
    /// (the mockup; the GTW-277 widget migration had dropped this label, leaving the bare
    /// switch). A static caption (not mutated): presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimLabel;
}

crate::support_item! {
    /// Marks the weapon panel's **name** [`Text`](bevy::prelude::Text) line — the selected
    /// weapon's [`WeaponName`](gdtf_battle_sim::weapon::WeaponName), mutated in place by the update.
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponNameText;
}

crate::support_item! {
    /// Marks the weapon panel's **magazine** [`Text`](bevy::prelude::Text) line — the
    /// `"cur/max"` round count (e.g. "30/30") from the
    /// [`Magazine`](gdtf_battle_sim::magazine::Magazine) grouping, mutated in place. Shown only when
    /// the weapon has a magazine (`size > 0`), else hidden. A unit marker: presence on an
    /// entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponMagazineText;
}

crate::support_item! {
    /// Marks the weapon panel's **LIVE Reload button** (GTW-275 — NOT a `DisabledButton`).
    /// A press pushes [`ActIntent::Reload`](gdtf_battle_input::ActIntent::Reload) onto the
    /// shared act-intent queue (the action-bar button precedent). Shown when the weapon has a
    /// magazine (`size > 0`), else [`Visibility::Hidden`] (mutated, never despawned). A unit
    /// marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ReloadButton;
}

//! Marker components for the DEV-ONLY procgen-visualizer INPUT PANEL (GTW-498).
//!
//! Each configurable control carries a named unit marker so the selection / commit listeners
//! (and the headless tests) find it by meaning rather than spawn order: the theme dropdown, the
//! three grid-axis numeric fields, the seed numeric field, the player / enemy gang dropdowns,
//! the Generate button, and the size-validity status text. Unit markers are the no-bare-types
//! "presence is the signal" form.
//!
//! The markers the headless tests name are declared through [`crate::support_item!`] so they
//! widen to `pub` under `test-support` and stay `pub(crate)` (binary `unreachable_pub`-clean)
//! otherwise. The whole module is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the THEME dropdown (C1) — a [`Dropdown`](gdtf_ui::Dropdown)`<`[`ThemeUuid`]`>`
    /// whose selection sets [`VizConfig::set_theme`].
    ///
    /// [`ThemeUuid`]: gdtf_battle_sim::level::ThemeUuid
    /// [`VizConfig::set_theme`]: super::resource::VizConfig::set_theme
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ThemeDropdown;
}

crate::support_item! {
    /// Marks the grid WIDTH numeric field (C2) — its commit sets [`VizConfig::set_width`].
    ///
    /// [`VizConfig::set_width`]: super::resource::VizConfig::set_width
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WidthField;
}

crate::support_item! {
    /// Marks the grid HEIGHT numeric field (C2) — its commit sets [`VizConfig::set_height`].
    ///
    /// [`VizConfig::set_height`]: super::resource::VizConfig::set_height
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HeightField;
}

crate::support_item! {
    /// Marks the grid LEVELS numeric field (C2) — its commit sets [`VizConfig::set_levels`].
    ///
    /// [`VizConfig::set_levels`]: super::resource::VizConfig::set_levels
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelsField;
}

crate::support_item! {
    /// Marks the SEED numeric field (C3) — its commit sets [`VizConfig::set_seed`].
    ///
    /// [`VizConfig::set_seed`]: super::resource::VizConfig::set_seed
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SeedField;
}

crate::support_item! {
    /// Marks the PLAYER-gang dropdown (C4) — a [`Dropdown`](gdtf_ui::Dropdown)`<`[`GangName`]`>`
    /// whose selection sets [`VizConfig::set_player_gang`].
    ///
    /// [`GangName`]: gdtf_battle_sim::ganger::GangName
    /// [`VizConfig::set_player_gang`]: super::resource::VizConfig::set_player_gang
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct PlayerGangDropdown;
}

crate::support_item! {
    /// Marks the ENEMY-gang dropdown (C4) — a [`Dropdown`](gdtf_ui::Dropdown)`<`[`GangName`]`>`
    /// whose selection sets [`VizConfig::set_enemy_gang`].
    ///
    /// [`GangName`]: gdtf_battle_sim::ganger::GangName
    /// [`VizConfig::set_enemy_gang`]: super::resource::VizConfig::set_enemy_gang
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EnemyGangDropdown;
}

crate::support_item! {
    /// Marks the GENERATE button (C5) — a press re-runs procgen from the current [`VizConfig`]
    /// and resets the reveal.
    ///
    /// [`VizConfig`]: super::resource::VizConfig
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct GenerateButton;
}

crate::support_item! {
    /// Marks the size-validity STATUS text (C2) — shows `OK` for a valid size, or the
    /// [`GridSizeError`](gdtf_battle_sim::level::GridSizeError) message for an invalid combo. The same
    /// validity drives the Generate button's enabled state, so an invalid size never regenerates.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SizeStatusText;
}

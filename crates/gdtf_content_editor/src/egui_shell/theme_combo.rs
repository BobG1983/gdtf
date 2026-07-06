//! The global-theme `ComboBox` option builder (GTW-512 C1) — the egui-native successor to the old
//! `bevy_ui` shell's `theme_options()`.
//!
//! Preserves that function's BEHAVIOR verbatim: one option per registered theme, labeled by its
//! display name, SORTED by label so the order is deterministic (the registry is a `HashMap`), with
//! index `0` the pre-selected default. An absent / empty registry yields an empty list (the
//! `ComboBox` then offers nothing rather than panicking). The only change is the option type — egui
//! has no equivalent of the old hand-rolled `DropdownOption`, so an option is a plain
//! `(label, ThemeUuid)` pair.

use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

/// One global-theme `ComboBox` option — a display label paired with the theme it selects. The
/// `ThemeUuid` is a domain key (the sim's own newtype), and the label is the theme's authored
/// display name, so neither leaf is a bare domain value (no-bare-types).
#[derive(Clone, Debug)]
pub(crate) struct ThemeOption {
    /// The theme's authored display name — the `ComboBox` row label.
    label: String,
    /// The theme key this option selects.
    key:   ThemeUuid,
}

impl ThemeOption {
    /// The `ComboBox` row label (the theme's display name).
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    /// The theme key this option selects.
    pub(crate) const fn key(&self) -> ThemeUuid {
        self.key
    }
}

/// Build the global-theme `ComboBox`'s option list from the [`UuidThemeRegistry`] — one
/// [`ThemeOption`] per registered theme, labeled by its display name, SORTED by label so the order
/// is deterministic (the registry is a `HashMap`). An absent / empty registry yields an empty list
/// (the `ComboBox` then offers nothing rather than panicking). Verbatim the BEHAVIOR of the old
/// `bevy_ui` shell's `theme_options()`.
pub(crate) fn theme_options(themes: Option<&UuidThemeRegistry>) -> Vec<ThemeOption> {
    let Some(themes) = themes else {
        return Vec::new();
    };
    let mut options: Vec<ThemeOption> = themes
        .defs()
        .map(|(key, def)| ThemeOption {
            label: (*def.display_name).clone(),
            key:   *key,
        })
        .collect();
    options.sort_by(|a, b| a.label.cmp(&b.label));
    options
}

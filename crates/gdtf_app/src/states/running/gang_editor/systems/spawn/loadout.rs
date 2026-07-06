//! Weapon and armor loadout helpers — dropdown option list builders for the member row.
//!
//! Separated from the row builder ([`super::row`]) so each concern has its own file. The
//! `sorted_*_options` functions are also consumed by
//! [`add_member_on_press`](super::super::add_member::add_member_on_press).

use bevy::prelude::*;
use gdtf_battle_sim::{ArmorName, ArmorRegistry, WeaponName, WeaponRegistry};
use gdtf_ui::{DropdownColors, DropdownOption, spawn_dropdown, theme::GdtfTheme};

use crate::states::{
    RunningState,
    running::gang_editor::components::{MemberArmorDropdown, MemberRowIndex, MemberWeaponDropdown},
};

/// Build the pre-sorted weapon dropdown option list — ALL loaded
/// [`WeaponName`](gdtf_battle_sim::WeaponName) keys (C2), sorted by name for a stable order (the
/// `HashMap` `keys()` order is unspecified). Each option's identity IS its key; its label is the
/// key string. Returns an empty list when the registry is absent (degraded, never panics).
pub(in crate::states::running::gang_editor) fn sorted_weapon_options(
    weapons: Option<&WeaponRegistry>,
) -> Vec<DropdownOption<WeaponName>> {
    let Some(weapons) = weapons else {
        return Vec::new();
    };
    let mut keys: Vec<&WeaponName> = weapons.keys().collect();
    keys.sort_by_key(|key| (***key).clone());
    keys.into_iter()
        .map(|key| DropdownOption::new(key.clone(), (**key).clone()))
        .collect()
}

/// Build the pre-sorted armor dropdown option list — ALL loaded
/// [`ArmorName`](gdtf_battle_sim::ArmorName) keys (C2), sorted by name. The armor mirror of
/// [`sorted_weapon_options`].
pub(in crate::states::running::gang_editor) fn sorted_armor_options(
    armor: Option<&ArmorRegistry>,
) -> Vec<DropdownOption<ArmorName>> {
    let Some(armor) = armor else {
        return Vec::new();
    };
    let mut keys: Vec<&ArmorName> = armor.keys().collect();
    keys.sort_by_key(|key| (***key).clone());
    keys.into_iter()
        .map(|key| DropdownOption::new(key.clone(), (**key).clone()))
        .collect()
}

/// The index of `id` in `options` (matched by identity), or `0` if absent — the dropdown's
/// initially-shown slot. A member whose current key is not a loaded option simply shows the first.
pub(super) fn option_index_of<T: gdtf_ui::OptionId>(
    options: &[DropdownOption<T>],
    id: &T,
) -> usize {
    options
        .iter()
        .position(|opt| opt.id() == id)
        .unwrap_or_default()
}

/// Spawn one row's WEAPON loadout control: the [`MemberWeaponDropdown`] over ALL loaded keys
/// (C1/C2). It carries the row's [`MemberRowIndex`] and opens on the member's current key (or the
/// first option if it is not a loaded key). The dropdown's own label shows the current key — there
/// is no separate echo text (GTW-499 C1: exactly one control per field). Returns the dropdown.
pub(super) fn spawn_weapon_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &WeaponName,
    options: &[DropdownOption<WeaponName>],
) -> Entity {
    spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberWeaponDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugGangEditor),
        ),
    )
}

/// Spawn one row's ARMOR loadout control: the [`MemberArmorDropdown`] over ALL loaded armor keys
/// (C1/C2). The armor mirror of [`spawn_weapon_loadout`] — the single armor control (GTW-499 C1).
/// Returns the dropdown.
pub(super) fn spawn_armor_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &ArmorName,
    options: &[DropdownOption<ArmorName>],
) -> Entity {
    spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberArmorDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugGangEditor),
        ),
    )
}

/// The [`DropdownColors`] a member-row dropdown paints with, from the theme. Pure UI plumbing.
pub(super) fn dropdown_colors(theme: &GdtfTheme) -> DropdownColors {
    DropdownColors {
        control_bg:          *theme.button.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.button.color,
        // The hovered / focused / selected option's highlight — the theme's hover fill, distinct
        // from the resting `option_bg` so the active row reads as highlighted (GTW-499).
        option_highlight_bg: *theme.button.hover,
    }
}

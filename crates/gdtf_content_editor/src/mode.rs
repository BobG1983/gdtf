//! The Workbench **editor-mode** machine (GTW-474; egui-swept GTW-512) — the [`EditorMode`]
//! resource + the number-key mode hotkeys.
//!
//! The editor is a multi-mode "Workbench": exactly one authoring workflow (one
//! [`TAB_ORDER`](EditorMode::TAB_ORDER) mode) is active at a time, held in the state-scoped
//! [`EditorMode`] resource (NOT a `SubStates` — bevy-traps #5), default
//! [`Prefab`](EditorMode::Prefab).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The pre-egui shell drove the mode through a `bevy_ui` segmented control (`apply_mode_switch`
//! reading `SegmentSelected`), toggled per-mode `Visibility`-containers (`toggle_mode_content`), and
//! synced the tab highlight to a hotkey (`sync_tabs_to_mode`). egui replaces all three: the
//! [`editor_egui_ui`](crate::egui_shell::editor_egui_ui) shell draws the mode TABS as egui
//! `selectable_value`s over this resource (a click sets it), branches the right panel on the active
//! mode in-UI (no `Visibility` containers), and reads the active mode for the status line. So the
//! `bevy_ui` mode systems + the tab / content markers are GONE; this module keeps the [`EditorMode`]
//! enum (+ its tab vocabulary the egui tabs reuse) and the UI-agnostic [`mode_hotkeys`].

use bevy::prelude::*;

/// The active **editor mode** — which authoring workflow the Workbench is showing (GTW-474).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), NOT a `SubStates` (it would trip bevy-traps #5). The egui shell renders one tab
/// per variant and branches its right panel on the active mode.
///
/// A named closed enum (no-bare-types: a mode is a domain value). The variant ORDER is the top-bar
/// tab order, so [`from_tab_index`](EditorMode::from_tab_index) / [`tab_index`](EditorMode::tab_index)
/// map a tab index to a mode and back. The default is [`Prefab`](EditorMode::Prefab) — the painter
/// the editor opened in before the Workbench grew around it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorMode {
    /// TERRAIN authoring — create a terrain definition (`*.terrain_def.ron`).
    Terrain,
    /// THEME authoring — assemble a theme definition (`*.terrain_theme.ron`) from the loaded
    /// terrain library (GTW-475).
    Theme,
    /// PREFAB authoring — paint a prefab with the selected theme (the original editor).
    #[default]
    Prefab,
    /// GANG authoring — edit a gang roster (`*.gang.ron`) and save it where the gangs
    /// folder loader reads (GTW-636; the USER RULING 2026-07-06: gangs are authored
    /// OUTSIDE the game binary — this mode replaces the retired in-game gang editor).
    Gang,
    /// ARMOR authoring — edit an armor suit's six per-body-part pieces (`*.armor.ron`)
    /// and save it where the armor folder loader reads (GTW-479; the GTW-636 Gang
    /// mode/form precedent).
    Armor,
    /// INJURY authoring — edit an injury def (`*.injury.ron`: severity / category /
    /// texts / the effects list) and the per-category weighting tables
    /// (`*.weighting.ron`), saving both where the bespoke injuries folder loader
    /// reads (GTW-654; the GTW-479 Armor mode/form precedent).
    Injury,
    /// SPRITE authoring — edit a sprite def (`*.spritedef.ron`: source / anchor /
    /// optional facings / optional animation) and save it where the GTW-663
    /// sprite-defs folder loader reads (GTW-664; the GTW-479 Armor mode/form
    /// precedent).
    Sprite,
    /// ATTACHMENT authoring — edit an attachment item (`*.attachment.ron`: display
    /// name / the closed 5-slot mount / the closed 13-effect list) and save it where
    /// the GTW-619 attachments folder loader reads (GTW-669; the GTW-479 Armor
    /// mode/form precedent).
    Attachment,
    /// WEAPON authoring — edit a full ranged `WeaponSpec` (`*.weapon.ron`: the seven
    /// stat scalars, the closed damage-type/handedness/trajectory vocabularies, the
    /// magazine, the fire-mode / slot / attachment lists, and the optional dot /
    /// on-death records) and save it where the GTW-257/570 ranged-weapons folder
    /// loader reads (GTW-670; the GTW-479 Armor mode/form precedent).
    Weapon,
    /// MELEE-WEAPON authoring — edit a full `MeleeWeaponSpec` (`*.melee_weapon.ron`:
    /// the six SHARED damage-group fields, the melee-only reach + fight-mode list, the
    /// shove tag, and the slot / attachment lists) and save it where the GTW-505/570
    /// melee-weapons folder loader reads (GTW-671; the GTW-670 Weapon mode/form
    /// precedent — the last GTW-478 child).
    MeleeWeapon,
}

impl EditorMode {
    /// The mode-tab order, left to right — the order the egui shell renders the tabs and the index
    /// order [`from_tab_index`](EditorMode::from_tab_index) maps. THEME sits BETWEEN Terrain and
    /// Prefab (GTW-475); GANG follows Prefab (GTW-636); ARMOR follows Gang (GTW-479); INJURY
    /// follows Armor (GTW-654); SPRITE follows Injury (GTW-664); ATTACHMENT follows Sprite
    /// (GTW-669); WEAPON follows Attachment (GTW-670); MELEE follows Weapon (GTW-671):
    /// `[TERRAIN | THEME | PREFAB | GANG | ARMOR | INJURY | SPRITE | ATTACHMENT | WEAPON | MELEE]`.
    pub const TAB_ORDER: [Self; 10] = [
        Self::Terrain,
        Self::Theme,
        Self::Prefab,
        Self::Gang,
        Self::Armor,
        Self::Injury,
        Self::Sprite,
        Self::Attachment,
        Self::Weapon,
        Self::MeleeWeapon,
    ];

    /// The mode at top-bar tab `index`, or [`None`] if the index is out of range — the inverse of
    /// [`tab_index`](EditorMode::tab_index).
    #[must_use]
    pub fn from_tab_index(index: usize) -> Option<Self> {
        Self::TAB_ORDER.get(index).copied()
    }

    /// This mode's position in the top-bar tab order — the index the active tab highlights.
    #[must_use]
    pub fn tab_index(self) -> usize {
        Self::TAB_ORDER
            .iter()
            .position(|mode| *mode == self)
            .unwrap_or(0)
    }

    /// The human label shown on this mode's top-bar tab (and in the status line).
    #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Terrain => "TERRAIN",
            Self::Theme => "THEME",
            Self::Prefab => "PREFAB",
            Self::Gang => "GANG",
            Self::Armor => "ARMOR",
            Self::Injury => "INJURY",
            Self::Sprite => "SPRITE",
            Self::Attachment => "ATTACHMENT",
            Self::Weapon => "WEAPON",
            Self::MeleeWeapon => "MELEE",
        }
    }
}

/// `Update` (in `Editing`): number-key hotkeys set the [`EditorMode`] (the `1`–`9` + `0`
/// mode hotkeys — `1`–`3` preserved across the egui swap, GTW-512 C1.3; `4` added with the
/// GANG mode, GTW-636; `5` with the ARMOR mode, GTW-479; `6` with the INJURY mode,
/// GTW-654; `7` with the SPRITE mode, GTW-664; `8` with the ATTACHMENT mode, GTW-669;
/// `9` with the WEAPON mode, GTW-670; `0` — the tenth tab — with the MELEE mode,
/// GTW-671).
///
/// `1` → [`Terrain`](EditorMode::Terrain), `2` → [`Theme`](EditorMode::Theme), `3` →
/// [`Prefab`](EditorMode::Prefab), `4` → [`Gang`](EditorMode::Gang), `5` →
/// [`Armor`](EditorMode::Armor), `6` → [`Injury`](EditorMode::Injury), `7` →
/// [`Sprite`](EditorMode::Sprite), `8` → [`Attachment`](EditorMode::Attachment), `9` →
/// [`Weapon`](EditorMode::Weapon), `0` → [`MeleeWeapon`](EditorMode::MeleeWeapon) — the
/// `[TERRAIN | THEME | PREFAB | GANG | ARMOR | INJURY | SPRITE | ATTACHMENT | WEAPON | MELEE]` tab order. Writes with
/// [`set_if_neq`](DetectChangesMut::set_if_neq) so an unchanged key-press is a no-op. Guarded on the
/// optional [`EditorMode`] (state-scoped — bevy-traps #1). UI-agnostic: the egui tabs and these keys
/// both write the same resource, and the next-frame egui draw reflects the change.
pub(crate) fn mode_hotkeys(keys: Res<ButtonInput<KeyCode>>, mode: Option<ResMut<EditorMode>>) {
    let Some(mut mode) = mode else {
        return;
    };
    let pressed = if keys.just_pressed(KeyCode::Digit1) {
        Some(EditorMode::Terrain)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(EditorMode::Theme)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(EditorMode::Prefab)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(EditorMode::Gang)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(EditorMode::Armor)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(EditorMode::Injury)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(EditorMode::Sprite)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(EditorMode::Attachment)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(EditorMode::Weapon)
    } else if keys.just_pressed(KeyCode::Digit0) {
        Some(EditorMode::MeleeWeapon)
    } else {
        None
    };
    if let Some(next) = pressed {
        mode.set_if_neq(next);
    }
}

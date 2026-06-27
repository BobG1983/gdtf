//! Spawns the gang-editor screen on `OnEnter(RunningState::DebugEditor)` (GTW-420 scaffold +
//! GTW-425 collapsed rows).
//!
//! Builds a themed panel layout holding:
//!
//! - a **title** heading,
//! - the gang-NAME text field via [`spawn_text_field`](gdtf_ui::spawn_text_field) (carrying the
//!   [`GangNameField`] marker so its commit maps to the model name — AC3),
//! - an **"Add member"** button via [`spawn_button`](gdtf_ui::spawn_button) (the
//!   [`AddMemberButton`] marker — AC4),
//! - the member-list SHELL: a GTW-412 [`ScrollList`](gdtf_ui::ScrollList) ([`MemberListHost`] on
//!   its root frame), seeded with one COLLAPSED [`MemberRow`] per member already in the model.
//!
//! Each collapsed row (GTW-425 C1) contains, left→right: an [`ExpandPip`] `+`/`-` toggle, a
//! [`MemberPortrait`] placeholder, the inline [`MemberNameField`] beside a [`MemberNameText`]
//! echo, a [`MemberWeaponText`] + a [`MemberWeaponDropdown`] over all loaded
//! [`WeaponName`](gdtf_battle_sim::WeaponName) keys, a [`MemberArmorText`] + a
//! [`MemberArmorDropdown`] over all loaded [`ArmorName`](gdtf_battle_sim::ArmorName) keys, and a
//! [`DeleteMemberButton`]. The EXPANDED per-member stat table the pip would gate is GTW-428 — out
//! of scope here; the pip renders its toggle state only.
//!
//! Every entity carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit)
//! so the whole screen tears down on leave (C1). The model resource is inserted by a sibling
//! system ([`insert_editable_gang`](super::model_lifecycle::insert_editable_gang)) ordered before
//! this one, so the member count is known when the shell is seeded.

use bevy::{prelude::*, scene::CommandsSceneExt, ui::Val};
use gdtf_battle_sim::{ArmorName, ArmorRegistry, WeaponName, WeaponRegistry};
use gdtf_ui::{
    ButtonLabel, CommittedTextValue, DropdownColors, DropdownOption, FieldColors, ScrollListColors,
    spawn_button, spawn_dropdown, spawn_panel, spawn_scroll_list, spawn_text_field,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::editor::{
        components::{
            AddMemberButton, DeleteMemberButton, EditorScreenRoot, ExpandPip, GangNameField,
            MemberArmorDropdown, MemberArmorText, MemberListHost, MemberNameField, MemberNameText,
            MemberPortrait, MemberRow, MemberRowIndex, MemberRowRef, MemberWeaponDropdown,
            MemberWeaponText, PipExpanded,
        },
        model::{EditableGang, EditableMember},
    },
};

/// Inter-child vertical gap of the editor column, in `Vh` — relative units (the responsive-UI
/// rule), reusing the menu's calibrated `10px / 720` separation so the editor spacing matches.
const EDITOR_GAP_VH: f32 = 1.388_89;

/// The collapsed `+` glyph the pip shows while a row is collapsed (the default — GTW-425 renders
/// the toggle state only; the expanded panel is GTW-428).
const PIP_COLLAPSED_GLYPH: &str = "+";

/// The portrait placeholder's side length, in viewport-width units — a small square colored block
/// standing in for the (not-yet-built) portrait system (C1). Relative, per the responsive-UI rule.
const PORTRAIT_SIDE_VW: f32 = 2.0;

/// Spawns the full editor screen when [`RunningState::DebugEditor`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time the menu can
/// reach the editor. Reads the [`EditableGang`] model (inserted by the ordered-before sibling
/// system) and the [`WeaponRegistry`] / [`ArmorRegistry`] (global resources the `Load` flow
/// inserts) as `Option<Res<…>>` so it is robust if any is absent. Each spawned node carries
/// [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit).
pub(in crate::states::running::editor) fn spawn_editor_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    model: Option<Res<EditableGang>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed screen (the menu precedent).
        return;
    };

    // ROOT — a centered, full-viewport themed backdrop column.
    let root = commands
        .spawn((
            EditorScreenRoot,
            Themed::new(ThemeRole::Background),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Vh(EDITOR_GAP_VH),
                ..default()
            },
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // TITLE — a themed heading, a direct child of the root.
    let title = commands
        .spawn_scene((
            bevy::scene::bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("GANG EDITOR")
            },
            bevy::scene::template_value(DespawnOnExit(RunningState::DebugEditor)),
        ))
        .id();

    // PANEL — wraps the editing controls in a themed box, a flex column.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::DebugEditor),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(EDITOR_GAP_VH),
            // A flex item that hosts a scroll list must allow itself to shrink below its content
            // (CSS `min-height: 0`) or the list bottom-cramps off-screen (the GTW-421 trap).
            min_height: Val::Px(0.0),
            ..default()
        },
    ));

    // GANG-NAME field — seeded with the model's current name, carrying the GangNameField marker
    // so the commit listener maps it to the model name (AC3).
    let initial_name = model
        .as_ref()
        .map(|model| model.name().as_str().to_owned())
        .unwrap_or_default();
    let name_field = spawn_text_field(
        &mut commands,
        CommittedTextValue::new(initial_name),
        field_colors(&theme),
        (GangNameField, DespawnOnExit(RunningState::DebugEditor)),
    );

    // ADD-MEMBER button (AC4).
    let add_button = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Add member"),
        (AddMemberButton, DespawnOnExit(RunningState::DebugEditor)),
    );

    // MEMBER-LIST shell — a GTW-412 scroll list. `spawn_scroll_list` puts the MemberListHost
    // marker on the grid ROOT FRAME and RETURNS the ScrollListArea (the clipping/scrolling
    // viewport) — the rows hang in the AREA, the FRAME is parented under the panel (the
    // GTW-421/422 scroll-list parenting rule). The frame is the area's parent once the builder's
    // internal `add_children` flushes; the panel hosts it via a deferred MemberListHost lookup so
    // we never depend on un-flushed parent links inside this builder.
    let list_area = spawn_scroll_list(
        &mut commands,
        scroll_list_colors(&theme),
        (MemberListHost, DespawnOnExit(RunningState::DebugEditor)),
    );

    // Seed one collapsed row per member already in the model (the load-from-registry path),
    // parented into the AREA (not the frame).
    let weapon_keys = sorted_weapon_options(weapons.as_deref());
    let armor_keys = sorted_armor_options(armor.as_deref());
    if let Some(model) = model.as_ref() {
        for (index, member) in model.members().iter().enumerate() {
            let row = spawn_member_row(
                &mut commands,
                &theme,
                index,
                member,
                &weapon_keys,
                &armor_keys,
            );
            commands.entity(list_area).add_child(row);
        }
    }

    commands
        .entity(panel)
        .add_children(&[name_field, add_button]);
    commands.entity(root).add_children(&[title, panel]);

    // Parent the scroll-list FRAME (the MemberListHost) under the panel and flex-size it, after
    // the builder's frame→area link has applied — a deferred lookup avoids depending on un-flushed
    // parent links AND on overwriting the builder's grid `Node` (we PATCH it, not replace it). The
    // frame fills the panel's remaining height (flex_grow) yet still shrinks to scroll
    // (`min_height: 0`, the GTW-421 bottom-cramp fix), rather than growing to its content.
    commands.queue(move |world: &mut World| {
        let Some(frame) = world
            .query_filtered::<Entity, With<MemberListHost>>()
            .iter(world)
            .next()
        else {
            return;
        };
        if let Some(mut node) = world.get_mut::<Node>(frame) {
            node.flex_grow = 1.0;
            node.min_height = Val::Px(0.0);
        }
        if let Ok(mut panel_entity) = world.get_entity_mut(panel) {
            panel_entity.add_child(frame);
        }
    });
}

/// Spawns ONE collapsed member ROW (GTW-425 C1) and returns its root [`Entity`].
///
/// The row is a flex ROW carrying its [`MemberRow`] marker + [`MemberRowIndex`], holding (in
/// order): the [`ExpandPip`] toggle, the [`MemberPortrait`] placeholder, the inline
/// [`MemberNameField`] + a [`MemberNameText`] echo, the [`MemberWeaponText`] + the
/// [`MemberWeaponDropdown`], the [`MemberArmorText`] + the [`MemberArmorDropdown`], and the
/// [`DeleteMemberButton`]. Every control carries the same [`MemberRowIndex`] so a commit /
/// selection / press maps back to the member. `weapon_options` / `armor_options` are the
/// pre-sorted dropdown option lists (all loaded keys — C2).
pub(in crate::states::running::editor) fn spawn_member_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    index: usize,
    member: &EditableMember,
    weapon_options: &[DropdownOption<WeaponName>],
    armor_options: &[DropdownOption<ArmorName>],
) -> Entity {
    let row_index = MemberRowIndex::new(index);
    let text_color = *theme.text.text_color;
    let portrait_color = *theme.panel.color;

    // The collapsed row root.
    let row = commands
        .spawn((
            MemberRow,
            row_index,
            Themed::new(ThemeRole::Panel),
            row_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // + pip — the expand-toggle CONTROL (renders its toggle state only; GTW-428 owns the panel).
    let pip = commands
        .spawn((
            ExpandPip,
            row_index,
            PipExpanded::new(false),
            Button,
            BackgroundColor(*theme.button.color),
            Text::new(PIP_COLLAPSED_GLYPH.to_owned()),
            TextColor(text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Portrait placeholder — a plain colored square (no portrait system yet).
    let portrait = commands
        .spawn((
            MemberPortrait,
            BackgroundColor(portrait_color),
            portrait_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Inline name field (C3), seeded with the member's current name, carrying the row index.
    let name_field = spawn_text_field(
        commands,
        CommittedTextValue::new(member.name().as_str().to_owned()),
        field_colors(theme),
        (
            MemberNameField,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    // Name echo text (C1) — what the dropdown/field commits MUTATE in place (C5). Carries the row
    // index so the commit system finds it.
    let name_text = commands
        .spawn((
            MemberNameText,
            row_index,
            Text::new(member.name().as_str().to_owned()),
            TextColor(text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Weapon name echo text + weapon dropdown over all loaded keys (C1/C2).
    let (weapon_text, weapon_dropdown) =
        spawn_weapon_loadout(commands, theme, row_index, member.weapon(), weapon_options);

    // Armor name echo text + armor dropdown over all loaded keys (C1/C2).
    let (armor_text, armor_dropdown) =
        spawn_armor_loadout(commands, theme, row_index, member.armor(), armor_options);

    // Delete button (C4) — carries the row index AND the row root so its press removes the right
    // member and despawns exactly this row.
    let delete = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Delete"),
        (
            DeleteMemberButton,
            row_index,
            MemberRowRef::new(row),
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );

    commands.entity(row).add_children(&[
        pip,
        portrait,
        name_field,
        name_text,
        weapon_text,
        weapon_dropdown,
        armor_text,
        armor_dropdown,
        delete,
    ]);
    row
}

/// Spawn one row's WEAPON loadout column: the [`MemberWeaponText`] echo (the member's current
/// weapon key) beside the [`MemberWeaponDropdown`] over ALL loaded keys (C1/C2). Both carry the
/// row's [`MemberRowIndex`]; the dropdown opens on the member's current key (or the first option if
/// it is not a loaded key). Returns `(text, dropdown)`.
fn spawn_weapon_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &WeaponName,
    options: &[DropdownOption<WeaponName>],
) -> (Entity, Entity) {
    let text = commands
        .spawn((
            MemberWeaponText,
            row_index,
            Text::new((**current).clone()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let dropdown = spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberWeaponDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    (text, dropdown)
}

/// Spawn one row's ARMOR loadout column: the [`MemberArmorText`] echo beside the
/// [`MemberArmorDropdown`] over ALL loaded armor keys (C1/C2). The armor mirror of
/// [`spawn_weapon_loadout`]. Returns `(text, dropdown)`.
fn spawn_armor_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &ArmorName,
    options: &[DropdownOption<ArmorName>],
) -> (Entity, Entity) {
    let text = commands
        .spawn((
            MemberArmorText,
            row_index,
            Text::new((**current).clone()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let dropdown = spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberArmorDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    (text, dropdown)
}

/// Build the pre-sorted weapon dropdown option list — ALL loaded
/// [`WeaponName`](gdtf_battle_sim::WeaponName) keys (C2), sorted by name for a stable order (the
/// `HashMap` `keys()` order is unspecified). Each option's identity IS its key; its label is the
/// key string. Returns an empty list when the registry is absent (degraded, never panics).
pub(in crate::states::running::editor) fn sorted_weapon_options(
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
pub(in crate::states::running::editor) fn sorted_armor_options(
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
fn option_index_of<T: gdtf_ui::OptionId>(options: &[DropdownOption<T>], id: &T) -> usize {
    options
        .iter()
        .position(|opt| opt.id() == id)
        .unwrap_or_default()
}

/// The [`FieldColors`] a text field paints with, read from the theme (background / text from the
/// panel + body-text sub-themes, the caret from the body text). Pure UI plumbing colors.
fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    }
}

/// The [`DropdownColors`] a member-row dropdown paints with, from the theme. Pure UI plumbing.
fn dropdown_colors(theme: &GdtfTheme) -> DropdownColors {
    DropdownColors {
        control_bg: *theme.button.color,
        text:       *theme.text.text_color,
        popup_bg:   *theme.panel.color,
        option_bg:  *theme.button.color,
    }
}

/// The [`ScrollListColors`] the member-list scroll list paints with, from the theme. Pure UI
/// plumbing.
fn scroll_list_colors(theme: &GdtfTheme) -> ScrollListColors {
    ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.button.color,
        thumb: *theme.text.text_color,
    }
}

/// One collapsed row's [`Node`]: a full-width flex ROW laying its controls left→right, centred on
/// the cross axis, with a small gap + padding so the controls read as one row. Relative units.
fn row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Vw(0.6),
        padding: bevy::ui::UiRect::all(Val::Vh(0.6)),
        ..default()
    }
}

/// The portrait placeholder's [`Node`]: a small square sized in viewport-relative units (the
/// palette tile-sprite precedent), so it scales with the window.
fn portrait_node() -> Node {
    Node {
        width: Val::Vw(PORTRAIT_SIDE_VW),
        height: Val::Vw(PORTRAIT_SIDE_VW),
        ..default()
    }
}

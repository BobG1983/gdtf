//! The member row builder — [`spawn_member_row`] and its node helpers.
//!
//! Separated from the shell ([`super::shell`]), the stat panel ([`super::stat_panel`]), and the
//! loadout helpers ([`super::loadout`]) so the collapsed-row layout has its own focused file.

use bevy::{prelude::*, ui::Val};
use gdtf_battle_sim::{ArmorName, GangerStatTuning, WeaponName};
use gdtf_ui::{
    ButtonLabel, CommittedTextValue, DropdownOption, spawn_button, spawn_text_field,
    theme::GdtfTheme,
};

use super::{
    loadout::{spawn_armor_loadout, spawn_weapon_loadout},
    stat_panel::{field_colors, spawn_member_stat_panel},
};
use crate::states::{
    RunningState,
    running::editor::{
        components::{
            DeleteMemberButton, ExpandPip, MemberNameField, MemberNameText, MemberPortrait,
            MemberRow, MemberRowIndex, MemberRowRef, PipExpanded,
        },
        model::EditableMember,
    },
};

/// The collapsed `+` glyph the pip shows while a row is collapsed (the default — GTW-425 renders
/// the toggle state only; the expanded panel is GTW-428).
const PIP_COLLAPSED_GLYPH: &str = "+";

/// The portrait placeholder's side length, in viewport-width units — a small square colored block
/// standing in for the (not-yet-built) portrait system (C1). Relative, per the responsive-UI rule.
const PORTRAIT_SIDE_VW: f32 = 2.0;

/// Spawns ONE member ROW (GTW-425 collapsed row + GTW-428 expanded stat panel) and returns its
/// root [`Entity`].
///
/// The row is a flex COLUMN: a collapsed HEADER row (the GTW-425 controls) above the GTW-428
/// expanded [`MemberStatPanel`]. The header row carries its [`MemberRow`] marker + [`MemberRowIndex`]
/// and holds (in order): the [`ExpandPip`] toggle, the [`MemberPortrait`] placeholder, the inline
/// [`MemberNameField`] + a [`MemberNameText`] echo, the [`MemberWeaponText`] + the
/// [`MemberWeaponDropdown`], the [`MemberArmorText`] + the [`MemberArmorDropdown`], and the
/// [`DeleteMemberButton`]. BELOW it sits the collapsed-by-default stat panel (C1) holding the eight
/// editable [`AttributeField`](crate::states::running::editor::components::AttributeField)s + the
/// readonly [`DerivedStatText`](crate::states::running::editor::components::DerivedStatText)
/// displays. Every control carries the same [`MemberRowIndex`] so a commit / selection / press maps
/// back to the member. `weapon_options` / `armor_options` are the pre-sorted dropdown option lists
/// (all loaded keys — C2); `tuning` is the GTW-384 derivation weights the panel seeds its readonly
/// displays from (C3).
pub(in crate::states::running::editor) fn spawn_member_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    index: usize,
    member: &EditableMember,
    weapon_options: &[DropdownOption<WeaponName>],
    armor_options: &[DropdownOption<ArmorName>],
    tuning: &GangerStatTuning,
) -> Entity {
    let row_index = MemberRowIndex::new(index);
    let text_color = *theme.text.text_color;
    let portrait_color = *theme.panel.color;

    // The member ROOT — a flex COLUMN: the collapsed header row above the expandable stat panel,
    // so the accordion lerp pushes the rows below it DOWN as the panel grows (C1).
    let row = commands
        .spawn((
            MemberRow,
            row_index,
            gdtf_ui::themed::Themed::new(gdtf_ui::themed::ThemeRole::Panel),
            member_root_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    // The collapsed HEADER row — the GTW-425 controls laid left→right.
    let header = commands
        .spawn((row_node(), DespawnOnExit(RunningState::DebugEditor)))
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

    commands.entity(header).add_children(&[
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

    // The GTW-428 expanded stat panel — the accordion content the pip lerps open / closed (C1),
    // holding the eight editable attribute fields + the readonly derived displays (C2/C3). Spawned
    // collapsed; below the header so its growth pushes later rows DOWN.
    let panel = spawn_member_stat_panel(commands, theme, row_index, member, tuning);

    commands.entity(row).add_children(&[header, panel]);
    row
}

/// One member's ROOT [`Node`]: a full-width flex COLUMN stacking the collapsed header row above
/// the expandable stat panel, so the accordion lerp grows the panel downward and pushes the rows
/// below it DOWN (C1). Relative units.
fn member_root_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// One collapsed HEADER row's [`Node`]: a full-width flex ROW laying its controls left→right,
/// centred on the cross axis, with a small gap + padding so the controls read as one row. Relative
/// units.
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

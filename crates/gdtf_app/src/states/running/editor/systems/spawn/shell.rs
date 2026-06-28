//! The editor screen shell — [`spawn_editor_screen`] and its debug-only
//! [`spawn_save_button`] helper.
//!
//! Separated from the member row builder ([`super::row`]), stat panel ([`super::stat_panel`]),
//! and loadout helpers ([`super::loadout`]) so the top-level screen layout has its own focused
//! file.

use bevy::{prelude::*, scene::CommandsSceneExt, ui::Val};
use gdtf_battle_sim::{ArmorRegistry, GangerStatTuning, WeaponRegistry};
use gdtf_ui::{
    ButtonLabel, CommittedTextValue, FieldColors, ScrollListColors, spawn_button, spawn_panel,
    spawn_scroll_list, spawn_text_field,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{
    loadout::{sorted_armor_options, sorted_weapon_options},
    row::spawn_member_row,
};
// The GTW-429 Save-gang button marker — spawned only in a debug build (C3/C4).
#[cfg(debug_assertions)]
use crate::states::running::editor::components::SaveGangButton;
use crate::states::{
    RunningState,
    running::editor::{
        components::{AddMemberButton, EditorScreenRoot, GangNameField, MemberListHost},
        model::EditableGang,
    },
};

/// Inter-child vertical gap of the editor column, in `Vh` — relative units (the responsive-UI
/// rule), reusing the menu's calibrated `10px / 720` separation so the editor spacing matches.
const EDITOR_GAP_VH: f32 = 1.388_89;

/// Spawns the full editor screen when [`RunningState::DebugEditor`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time the menu can
/// reach the editor. Reads the [`EditableGang`] model (inserted by the ordered-before sibling
/// system), the [`WeaponRegistry`] / [`ArmorRegistry`] (global resources the `Load` flow
/// inserts), and the [`GangerStatTuning`] (the GTW-384 derivation weights, a persistent resource
/// the `Load` flow inserts) as `Option<Res<…>>` so it is robust if any is absent — an absent
/// tuning seeds the readonly derived displays with the const-default derivation (C3). Each spawned
/// node carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit).
pub(in crate::states::running::editor) fn spawn_editor_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    model: Option<Res<EditableGang>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    tuning: Option<Res<GangerStatTuning>>,
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
    // Resolve the derivation tuning once (cloned so each row's seed can re-derive); an absent
    // resource falls back to the const-default weights (C3).
    let tuning = tuning.as_deref().cloned().unwrap_or_default();
    if let Some(model) = model.as_ref() {
        for (index, member) in model.members().iter().enumerate() {
            let row = spawn_member_row(
                &mut commands,
                &theme,
                index,
                member,
                &weapon_keys,
                &armor_keys,
                &tuning,
            );
            commands.entity(list_area).add_child(row);
        }
    }

    commands
        .entity(panel)
        .add_children(&[name_field, add_button]);
    // Spawn + parent the debug-only "Save gang" button into the panel beside Add member (GTW-429
    // C4); a release build never compiles it (matching the debug-only save system — C3).
    #[cfg(debug_assertions)]
    spawn_save_button(&mut commands, &theme, panel);
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

/// Spawns the "Save gang" button and parents it into the editor `panel` beside "Add member"
/// (GTW-429 C4) — `#[cfg(debug_assertions)]` ONLY, matching the debug-only save system (C3).
///
/// Extracted from [`spawn_editor_screen`] so that function stays under the `too_many_lines` lint;
/// it carries the [`SaveGangButton`] marker the
/// [`save_gang_on_press`](super::super::save::save_gang_on_press) system reads, plus
/// [`DespawnOnExit`] so it tears down with the rest of the screen on leave.
#[cfg(debug_assertions)]
fn spawn_save_button(commands: &mut Commands, theme: &GdtfTheme, panel: Entity) {
    let save_button = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Save gang"),
        (SaveGangButton, DespawnOnExit(RunningState::DebugEditor)),
    );
    commands.entity(panel).add_child(save_button);
}

/// The [`FieldColors`] a text field paints with, read from the theme (background / text from the
/// panel + body-text sub-themes, the caret from the body text). Pure UI plumbing colors.
pub(super) fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
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

//! The **Combined Weapon Panel** builders (top-left grid cell): the full-width
//! weapon-image placeholder over the weapon-text / Reload info row. Split out of the
//! monolithic `spawn.rs` (GTW-583); the authoritative layout doc lives on the parent
//! `spawn` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Display, Node, Overflow, OverflowAxis, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, spawn_button,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::geometry::{CONTENT_MIN_H_VH, GAP_VH, GAP_VW, INFO_ROW_H_PCT, TEXT_COL_PCT};
use crate::states::running::game::battlescape::weapon_panel::components::{
    CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage, WeaponMagazineText,
    WeaponNameText,
};

/// Builds the FULL-WIDTH [`WeaponImage`] placeholder — a framed box with a small centered
/// "no image" caption, spanning the Combined panel's full width at `height` (a [`Val::Percent`]
/// of the panel's top half). Standing in for per-weapon art that does NOT exist.
fn spawn_image(commands: &mut Commands, theme: &GdtfTheme, height: Val) -> Entity {
    // GTW-322 — `WeaponImage` + `Themed` ride the `bsn!` macro inline; the runtime-valued
    // `Node`, `BackgroundColor`, and `BorderColor` are composed with `template_value`.
    let slot_node = Node {
        width: Val::Percent(100.0),
        height,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let slot = commands
        .spawn_scene((
            bsn! {
                WeaponImage
                Themed::new(ThemeRole::Panel)
            },
            template_value(slot_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    // The "no image" caption: `Themed` + `Text` + `UiTextColor` inline, `TextFont` (not
    // `Unpin`) on the `template(|_| ..)` closure.
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    let label = commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new("no image")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id();
    commands.entity(slot).add_children(&[label]);
    slot
}

/// Spawns a themed [`Text`] line tagged `marker`, started at `initial` (the stat-block
/// `spawn_text` precedent). A NON-EMPTY `initial` seeds the first-frame measure; the update
/// mutates the text in place.
fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    // GTW-322 — `Themed` + `Text::new` + `UiTextColor` ride the `bsn!` macro inline;
    // `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure; the generic `marker`
    // is `.insert`ed. The seed string is owned (`'static`) for the deferred apply.
    let text_color = *theme.text.text_color;
    let caption = initial.to_owned();
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}

/// Builds the Combined panel's INFO ROW (bottom 1/2) — the [`WeaponContent`] weapon-text column
/// (name + magazine, 3/4 width) beside the LIVE [`ReloadButton`] (1/4 width).
///
/// The text column `flex_grow`s / CLIPS (`min_width: 0` + [`Overflow`] hidden) rather than
/// pushing Reload; the Reload cell is `flex_shrink: 0` (never collapses) and pinned to the
/// row's RIGHT END, so Reload can never float past the Combined panel's right edge into the Item
/// panel (contract: Reload INSIDE the Combined panel). Returns the info-row entity.
fn spawn_info_row(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let name = spawn_text(commands, theme, WeaponNameText, "—");
    let magazine = spawn_text(commands, theme, WeaponMagazineText, "0/0");
    // GTW-322 — `WeaponContent` rides the `bsn!` macro inline; its runtime-valued `Node`
    // (the flex-sponge text column) + `Visibility::Hidden` are composed with `template_value`.
    let content_node = Node {
        width: Val::Percent(TEXT_COL_PCT),
        height: Val::Percent(100.0),
        // The GTW-275 overflow floor: a RESPONSIVE (`Val::Vh`) min height so the
        // auto-sized panel cannot mismeasure the weapon-text against near-zero text.
        min_height: Val::Vh(CONTENT_MIN_H_VH),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_width: Val::ZERO,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        // Hidden as a unit when there is no selection / no weapon (AC9 empty state):
        // Display::None (removed from layout) + Visibility::Hidden. The update reveals it.
        display: Display::None,
        ..default()
    };
    let content = commands
        .spawn_scene((
            bsn! { WeaponContent },
            template_value(content_node),
            template_value(Visibility::Hidden),
        ))
        .id();
    commands.entity(content).add_children(&[name, magazine]);

    let reload = spawn_button(commands, theme, ButtonLabel::new("Reload"), ReloadButton);
    // GTW-322 — plain layout `Node`s (no markers); composed via `template_value`.
    let reload_cell_node = Node {
        width: Val::Percent(100.0 - TEXT_COL_PCT),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        align_items: AlignItems::Center,
        ..default()
    };
    let reload_cell = commands.spawn_scene(template_value(reload_cell_node)).id();
    commands.entity(reload_cell).add_children(&[reload]);

    let info_row_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(INFO_ROW_H_PCT),
        flex_direction: FlexDirection::Row,
        column_gap: Val::Vw(GAP_VW),
        align_items: AlignItems::Center,
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        ..default()
    };
    let info_row = commands.spawn_scene(template_value(info_row_node)).id();
    commands
        .entity(info_row)
        .add_children(&[content, reload_cell]);
    info_row
}

/// Builds the **Combined Weapon Panel** (top-left grid cell) — ONE bordered box of the
/// FULL-WIDTH [`WeaponImage`] (top 1/2) over the info row (bottom 1/2: weapon-text column |
/// Reload). Sized `width` × `height` (responsive). Returns the panel [`Entity`].
pub(super) fn spawn_combined_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let image = spawn_image(commands, theme, Val::Percent(INFO_ROW_H_PCT));
    let info_row = spawn_info_row(commands, theme);
    // GTW-322 — `CombinedWeaponPanel` + `Themed` ride the `bsn!` macro inline; the
    // runtime-valued `Node`, `BackgroundColor`, and `BorderColor` are composed with
    // `template_value`.
    let panel_node = Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let panel = commands
        .spawn_scene((
            bsn! {
                CombinedWeaponPanel
                Themed::new(ThemeRole::Panel)
            },
            template_value(panel_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    commands.entity(panel).add_children(&[image, info_row]);
    panel
}

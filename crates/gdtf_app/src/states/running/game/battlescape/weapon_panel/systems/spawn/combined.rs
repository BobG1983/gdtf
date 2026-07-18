//! The **Combined Weapon Panel** builders (top-left grid cell): the full-width
//! weapon-image placeholder over the weapon-text block STACKED OVER the Reload row (GTW-733 —
//! Reload got its own row below the name/magazine text rather than sharing a row beside it, so
//! the two never occupy the same pixels). Split out of the monolithic `spawn.rs` (GTW-583); the
//! authoritative layout doc lives on the parent `spawn` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, LineBreak, TextColor as UiTextColor, TextFont, TextLayout},
    ui::{Display, Node, Overflow, OverflowAxis, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, spawn_button,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::geometry::{
    CONTENT_MIN_H_VH, GAP_VH, INFO_RELOAD_H_PCT, INFO_ROW_H_PCT, INFO_TEXT_H_PCT,
};
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

/// Builds the Combined panel's INFO BLOCK (bottom 1/2) — the [`WeaponContent`] weapon-text
/// block (name + magazine, FULL width, top [`INFO_TEXT_H_PCT`] share) STACKED OVER the LIVE
/// [`ReloadButton`]'s OWN row (full width, bottom [`INFO_RELOAD_H_PCT`] share).
///
/// GTW-733 (weapon-block-too-narrow bug fix): Reload previously shared a ROW with the
/// weapon-text column (a 3/4-width text column beside a 1/4-width Reload cell), so a long
/// shipped identifier (e.g. `grenade_launcher`, `volatile_charge` — `snake_case`, so it has no
/// space `bevy_text` can word-wrap at) had nowhere to go but INTO Reload's cell. Stacking Reload
/// into its OWN row below the text — rather than beside it — means the name row and the Reload
/// button never share the same pixels, REGARDLESS of how long a shipped name gets: the two are
/// disjoint rects by construction, not by hoping the name fits. The text block still `Overflow`s
/// hidden + wraps [`LineBreak::WordOrCharacter`] (so an even longer future name breaks onto a
/// second line inside its OWN box rather than bleeding out of it) as the graceful-degradation
/// fallback under [`CONTENT_MIN_H_VH`]'s two-line floor. Returns the info-block entity.
fn spawn_info_block(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let name = spawn_text(commands, theme, WeaponNameText, "—");
    // GTW-733: the name may be a long, space-free snake_case identifier the default
    // `LineBreak::WordBoundary` cannot wrap at all (no break opportunity) — `WordOrCharacter`
    // wraps at a word boundary when one exists, else falls back to breaking mid-word, so an
    // over-long name degrades to a second line INSIDE its own box instead of overflowing past it.
    commands
        .entity(name)
        .insert(TextLayout::linebreak(LineBreak::WordOrCharacter));
    let magazine = spawn_text(commands, theme, WeaponMagazineText, "0/0");
    // GTW-322 — `WeaponContent` rides the `bsn!` macro inline; its runtime-valued `Node`
    // (the FULL-WIDTH text block, GTW-733) + `Visibility::Hidden` are composed with
    // `template_value`.
    let content_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(INFO_TEXT_H_PCT),
        // The GTW-275 overflow floor (doubled by GTW-733 to hold a two-line wrapped name): a
        // RESPONSIVE (`Val::Vh`) min height so the auto-sized panel cannot mismeasure the
        // weapon-text against near-zero text.
        min_height: Val::Vh(CONTENT_MIN_H_VH),
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
    // GTW-733 — Reload's OWN full-width row (below the text block, not beside it); the button
    // itself stays RIGHT-anchored within it, matching its prior position in the row.
    // GTW-322 — plain layout `Node` (no marker); composed via `template_value`.
    let reload_row_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(INFO_RELOAD_H_PCT),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        align_items: AlignItems::Center,
        ..default()
    };
    let reload_row = commands.spawn_scene(template_value(reload_row_node)).id();
    commands.entity(reload_row).add_children(&[reload]);

    // GTW-733: a COLUMN (was a Row) — the text block stacks OVER the Reload row, so the two
    // never occupy the same pixels.
    let info_block_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(INFO_ROW_H_PCT),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        ..default()
    };
    let info_block = commands.spawn_scene(template_value(info_block_node)).id();
    commands
        .entity(info_block)
        .add_children(&[content, reload_row]);
    info_block
}

/// Builds the **Combined Weapon Panel** (top-left grid cell) — ONE bordered box of the
/// FULL-WIDTH [`WeaponImage`] (top 1/2) over the info block (bottom 1/2: the FULL-WIDTH
/// weapon-text block stacked over Reload's own row, GTW-733). Sized `width` × `height`
/// (responsive). Returns the panel [`Entity`].
pub(super) fn spawn_combined_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let image = spawn_image(commands, theme, Val::Percent(INFO_ROW_H_PCT));
    let info_block = spawn_info_block(commands, theme);
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
    commands.entity(panel).add_children(&[image, info_block]);
    panel
}

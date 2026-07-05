//! The right-column widgets: the shared framed-box builder, the Aim caption, and the
//! Item panel with its disabled item buttons. Split out of the monolithic `spawn.rs`
//! (GTW-583); the authoritative layout doc lives on the parent `spawn` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, DisabledButton, spawn_button,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::geometry::GAP_VH;
use crate::states::running::game::battlescape::weapon_panel::components::{
    AimLabel, WeaponItemButton, WeaponItemPanel,
};

/// Builds an EMPTY FRAMED placeholder box tagged `marker`, sized `width` × `height`.
///
/// A `Themed(ThemeRole::Panel)` box (a themed border / bg / radius re-painted by `apply_theme`
/// like any themed node, so it reads as a framed box). Used for the Combined panel, the Item
/// panel, the Aim panel, and the image placeholder shell. Returns its [`Entity`].
pub(super) fn spawn_frame(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    width: Val,
    height: Val,
) -> Entity {
    // GTW-322 — `Themed` rides the `bsn!` macro inline; the runtime-valued `Node`,
    // `BackgroundColor`, and `BorderColor` are composed with `template_value` (the
    // builder `BorderColor::all` is a method call, which the inline `CompA(expr)` grammar
    // rejects, so it is precomputed into a value); the generic `marker` is `.insert`ed.
    let node = Node {
        width,
        height,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Panel) },
            template_value(node),
            template_value(background),
            template_value(border),
        ))
        .insert(marker)
        .id()
}

/// Spawns the Aim Panel's **"Aim" caption** [`Text`] ([`AimLabel`]) — the static label that sits
/// to the LEFT of the relocated Aim [`Switch`](gdtf_ui::Switch) so the control reads
/// "Aim [switch]" (the mockup; the GTW-277 widget migration had dropped this caption). Returns
/// its [`Entity`].
///
/// A `Themed(ThemeRole::Text)` line (the "no image" caption precedent), so `apply_theme` paints
/// its font + color from the theme like any themed text. Its [`AimLabel`] marker is added to
/// [`fit_weapon_panel`](super::super::fit::fit_weapon_panel)'s label-owner set, which holds the caption
/// at the 14 pt control-label size `.after(UiSystems::ApplyTheme)` — matching the firemode /
/// stance segment captions (`nowrap_control_labels`) so the whole control cluster's labels read
/// at one size. A non-empty seed feeds the first-frame measure; the caption is static (never
/// mutated).
pub(super) fn spawn_aim_label(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    // GTW-322 — `AimLabel` + `Themed` + `Text` + `UiTextColor` ride the `bsn!` macro
    // inline; `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure.
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            AimLabel
            Themed::new(ThemeRole::Text)
            Text::new("Aim")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id()
}

/// Builds the **Item Panel** (top-right grid cell) — a framed box holding two stacked DISABLED
/// [`WeaponItemButton`]s (1/2 height each, full width). Sized `width` × `height`. Returns the
/// panel [`Entity`].
pub(super) fn spawn_item_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let panel = spawn_frame(commands, theme, WeaponItemPanel, width, height);
    let item_a = spawn_item_button(commands, theme);
    let item_b = spawn_item_button(commands, theme);
    commands.entity(panel).insert(Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    });
    commands.entity(panel).add_children(&[item_a, item_b]);
    panel
}

/// Spawns one DISABLED [`WeaponItemButton`] (full width, 1/2 height of the Item panel) — a
/// visible-but-non-interactable placeholder (items are not modeled yet). It carries
/// [`DisabledButton`](gdtf_ui::DisabledButton) so `gdtf_ui` paints it in the disabled color and
/// the `Without<DisabledButton>` interaction filters exclude it. Returns the button.
fn spawn_item_button(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let button = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Item"),
        (WeaponItemButton, DisabledButton),
    );
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_basis: Val::Percent(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}

//! The reusable theme-seam spawn helpers: [`spawn_panel`] and [`spawn_button`].
//!
//! The helpers build a widget's *tree* (its [`Node`](bevy::ui::Node) layout, its
//! [`Button`](bevy::ui::Button) interaction plumbing, its text child) and attach
//! the [`Themed`](crate::themed::Themed) marker (GTW-135). They write *initial*
//! theme-derived colors so a widget is never un-themed for a frame, but those
//! colors are **not** authoritative: the central
//! [`apply_theme`](crate::themed::apply_theme) system re-derives and re-writes them
//! from the live [`GdtfTheme`](crate::theme::GdtfTheme) on every run.

use bevy::{
    prelude::*,
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use super::markers::ButtonLabel;
use crate::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

/// Spawns a theme-seam panel box and returns its [`Entity`].
///
/// Builds a [`Node`](bevy::ui::Node) with a
/// [`BackgroundColor`](bevy::ui::BackgroundColor),
/// [`BorderColor`](bevy::ui::BorderColor), and
/// [`BorderRadius`](bevy::ui::BorderRadius)-bearing node, and attaches
/// [`Themed(ThemeRole::Panel)`](crate::themed::Themed). The initial colors,
/// border width, corner radius, and content-margin padding are all read from the
/// **panel** sub-theme of `theme` — never literals.
///
/// The colors written here are *initial*, not frozen:
/// [`apply_theme`](crate::themed::apply_theme) re-derives the identical look from
/// the live [`GdtfTheme`](crate::theme::GdtfTheme) every run, so re-running it
/// reproduces them and a hot-reload re-paints the panel.
pub fn spawn_panel(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let node = box_node(
        *theme.panel.border_width,
        *theme.panel.corner_radius,
        theme,
        BoxKind::Panel,
    );
    commands
        .spawn((
            Themed::new(ThemeRole::Panel),
            node,
            BackgroundColor(*theme.panel.color),
            UiBorderColor::all(*theme.panel.border_color),
        ))
        .id()
}

/// Spawns a theme-seam button and returns its [`Entity`].
///
/// Builds a [`Button`](bevy::ui::Button) tree — a node with
/// [`Interaction`](bevy::ui::Interaction) (required by `Button`),
/// [`BackgroundColor`](bevy::ui::BackgroundColor),
/// [`BorderColor`](bevy::ui::BorderColor), and
/// [`BorderRadius`](bevy::ui::BorderRadius) — carrying a
/// [`Text`](bevy::prelude::Text) child with the **button** sub-theme's
/// [`TextFont`](bevy::text::TextFont) (its resolved
/// [`Handle<Font>`](bevy::prelude::Handle) at its `font_size_pt`) and
/// [`TextColor`](bevy::text::TextColor). The root carries
/// [`Themed(ThemeRole::Button)`](crate::themed::Themed); the caption child carries
/// [`Themed(ThemeRole::ButtonText)`](crate::themed::Themed). The caller-supplied
/// `marker` bundle is added to the root.
///
/// `label` is the button caption; `marker` is any [`Bundle`] the caller wants on
/// the button (a focus/action marker, a [`DisabledButton`](super::DisabledButton),
/// etc.). All base colors come from the button sub-theme, not literals, and are
/// re-derived by [`apply_theme`](crate::themed::apply_theme) every run (the Themed
/// seam).
pub fn spawn_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: ButtonLabel,
    marker: impl Bundle,
) -> Entity {
    let font = theme.button.font.clone();
    let font_size = *theme.button.font_size_pt;
    let text_color = *theme.button.text_color;
    let node = box_node(
        *theme.button.border_width,
        *theme.button.corner_radius,
        theme,
        BoxKind::Button,
    );

    commands
        .spawn((
            Button,
            Themed::new(ThemeRole::Button),
            node,
            BackgroundColor(*theme.button.color),
            UiBorderColor::all(*theme.button.border_color),
            marker,
        ))
        .with_children(|parent| {
            parent.spawn((
                Themed::new(ThemeRole::ButtonText),
                Text::new(label.into_inner()),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                UiTextColor(text_color),
            ));
        })
        .id()
}

/// Which sub-theme's content-margin a [`box_node`] reads.
///
/// A panel and a button each carry their own padding in their own sub-theme; this
/// selects between them so the one builder serves both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BoxKind {
    /// Read the panel sub-theme's content margin.
    Panel,
    /// Read the button sub-theme's content margin.
    Button,
}

/// Builds the initial themed box [`Node`](bevy::ui::Node) for a panel or a button:
/// the theme border width, corner radius, and the selected sub-theme's content
/// padding. The colors are initial-only — [`apply_theme`](crate::themed::apply_theme)
/// re-derives them.
///
/// All sizes are RELATIVE units (GTW-296): the border width + corner radius are
/// `Vw` (one axis, so a border/radius pair keeps its ratio), and the per-edge
/// padding is `Vw` on the horizontal edges + `Vh` on the vertical, so each axis
/// tracks the matching window dimension on resize.
fn box_node(border_vw: f32, radius_vw: f32, theme: &GdtfTheme, kind: BoxKind) -> Node {
    let margin = match kind {
        BoxKind::Panel => theme.panel.margin,
        BoxKind::Button => theme.button.margin,
    };
    Node {
        border: UiRect::all(Val::Vw(border_vw)),
        border_radius: BorderRadius::all(Val::Vw(radius_vw)),
        padding: UiRect {
            left:   Val::Vw(*margin.l),
            right:  Val::Vw(*margin.r),
            top:    Val::Vh(*margin.t),
            bottom: Val::Vh(*margin.b),
        },
        ..default()
    }
}

//! Reusable, theme-seam widget builders and the disabled-button marker.
//!
//! This module owns the two spawn helpers the menu / HUD work builds its UI
//! from — [`spawn_panel`] and [`spawn_button`] — plus the [`DisabledButton`]
//! marker and the [`paint_disabled_buttons`] system that paints it.
//!
//! ## The Themed seam (no captured colors)
//!
//! The helpers build a widget's *tree* (its [`Node`](bevy::ui::Node) layout,
//! its [`Button`](bevy::ui::Button) interaction plumbing, its text child) and
//! attach the [`Themed`](crate::themed::Themed) marker (GTW-135). They write
//! *initial* theme-derived colors so a widget is never un-themed for a frame,
//! but those colors are **not** authoritative: the central
//! [`apply_theme`](crate::themed::apply_theme) system re-derives and re-writes
//! them from the live [`GdtfTheme`](crate::theme::GdtfTheme) on every run, so a
//! hot-reload re-paints every widget. A helper that froze colors at spawn would
//! break that seam; these deliberately do not.
//!
//! Post-GTW-149 a panel is painted from the **panel** sub-theme and a button from
//! the **button** sub-theme — they are now distinct boxes, not one shared "panel"
//! look. A button's caption child carries [`ThemeRole::ButtonText`], drawing the
//! button sub-theme's typography.
//!
//! ## Disabled buttons
//!
//! A [`DisabledButton`] is painted by [`paint_disabled_buttons`] with the
//! button sub-theme's explicit [`DisabledColor`](crate::theme::DisabledColor)
//! fill (re-applied after [`apply_theme`](crate::themed::apply_theme)) and is
//! skipped entirely by the interaction layer (which filters
//! `Without<DisabledButton>`). It stays [`Themed`](crate::themed::Themed), so the
//! base-look pass still reaches it.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use crate::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

/// Marker tagging a button that is currently disabled.
///
/// A disabled button is **painted** ([`paint_disabled_buttons`] writes the button
/// sub-theme's explicit [`DisabledColor`](crate::theme::DisabledColor) fill over
/// its base) and is **skipped** by the interaction layer (the interaction system
/// queries `Without<DisabledButton>`, so a disabled button never hover/press-swaps).
/// It remains [`Themed`](crate::themed::Themed): the central base-look pass still
/// re-paints it, and the disabled fill is composed on top of that fresh base.
///
/// A unit marker — it carries no data; presence alone is the signal
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisabledButton;

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
        *theme.panel.border_width_px,
        *theme.panel.corner_radius_px,
        theme,
        BoxKind::Panel,
    );
    commands
        .spawn((
            Themed(ThemeRole::Panel),
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
/// the button (a focus/action marker, a [`DisabledButton`], etc.). All base
/// colors come from the button sub-theme, not literals, and are re-derived by
/// [`apply_theme`](crate::themed::apply_theme) every run (the Themed seam).
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
        *theme.button.border_width_px,
        *theme.button.corner_radius_px,
        theme,
        BoxKind::Button,
    );

    commands
        .spawn((
            Button,
            Themed(ThemeRole::Button),
            node,
            BackgroundColor(*theme.button.color),
            UiBorderColor::all(*theme.button.border_color),
            marker,
        ))
        .with_children(|parent| {
            parent.spawn((
                Themed(ThemeRole::ButtonText),
                Text::new(label.into_inner()),
                TextFont {
                    font,
                    font_size,
                    ..default()
                },
                UiTextColor(text_color),
            ));
        })
        .id()
}

/// The visible caption of a [`spawn_button`] widget.
///
/// A named newtype over the caption string rather than a bare `String`
/// (no-bare-types rule): a button's label is a domain value, not arbitrary text.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct ButtonLabel(String);

impl ButtonLabel {
    /// Wraps a caption into a [`ButtonLabel`].
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// Consumes the label, yielding its inner caption string for
    /// [`Text::new`](bevy::prelude::Text::new).
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
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
fn box_node(border_px: f32, radius_px: f32, theme: &GdtfTheme, kind: BoxKind) -> Node {
    let margin = match kind {
        BoxKind::Panel => theme.panel.margin,
        BoxKind::Button => theme.button.margin,
    };
    Node {
        border: UiRect::all(Val::Px(border_px)),
        border_radius: BorderRadius::all(Val::Px(radius_px)),
        padding: UiRect::px(*margin.l, *margin.r, *margin.t, *margin.b),
        ..default()
    }
}

/// Paints every [`DisabledButton`] with the button sub-theme's explicit
/// [`DisabledColor`](crate::theme::DisabledColor) fill, writing it over the
/// button's [`BackgroundColor`](bevy::ui::BackgroundColor).
///
/// Runs **after** [`apply_theme`](crate::themed::apply_theme) (the
/// [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set) so it
/// composes on top of the freshest base fill (bevy-traps rule 3); a hot-reload
/// therefore re-paints the disabled fill from the new palette. The fill is the
/// live theme's data-driven [`DisabledColor`](crate::theme::DisabledColor) — a
/// muted color so a disabled control reads as inert.
///
/// Takes the theme as `Option<Res<GdtfTheme>>` so it is inert (rather than
/// panicking) before the resource is populated (bevy-traps rule 1).
pub fn paint_disabled_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut disabled: Query<&mut BackgroundColor, With<DisabledButton>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let fill = *theme.button.disabled;
    for mut background in &mut disabled {
        background.0 = fill;
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        text::TextColor as UiTextColor,
        ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, Node, widget::Button},
    };

    use super::*;
    use crate::{
        theme::{GdtfTheme, GdtfThemeSpec},
        themed::{ThemeRole, Themed, apply_theme},
    };

    /// Builds a [`GdtfTheme`] through the real resolution path (deserialize the
    /// nested spec, then [`GdtfThemeSpec::resolve`]) with a defaulted-font
    /// resolver. The button sub-theme's fill is caller-chosen so the paint tests
    /// can pin it. The theme newtypes have private fields, so this respects
    /// encapsulation while exercising the genuine runtime resolution. Returns the
    /// `ron` error so a malformed literal surfaces via `?`.
    fn theme(
        button_color: [f32; 4],
        disabled: [f32; 4],
    ) -> Result<GdtfTheme, ron::error::SpannedError> {
        let [pr, pg, pb, pa] = button_color;
        let [dr, dg, db, da] = disabled;
        let ron = format!(
            "(\
             default_font: \"fonts/test.ttf\", \
             background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
             panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                      border_width_px: 2.0, corner_radius_px: 5.0, \
                      margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
             button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: ({dr}, {dg}, {db}, {da}), \
                       hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                       text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0, \
                       border_color: (0.20, 0.20, 0.24, 1.0), \
                       border_width_px: 2.0, corner_radius_px: 5.0, \
                       margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
             title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
             text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
        );
        let spec: GdtfThemeSpec = ron::from_str(&ron)?;
        Ok(spec.resolve(|_| Handle::<Font>::default()))
    }

    /// `spawn_button` builds a button carrying the interaction plumbing
    /// (`Button` + `Interaction`), the button-box visuals (`BackgroundColor` +
    /// `BorderColor`), and the `Themed(Button)` marker.
    ///
    /// Pin-discriminating: dropping any of those components, or the Themed
    /// marker, fails an assert.
    #[test]
    fn spawned_button_has_interaction_visuals_and_themed_marker()
    -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        app.insert_resource(theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?);

        let theme_res = app.world().resource::<GdtfTheme>().clone();
        let button = {
            let mut commands = app.world_mut().commands();
            spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
        };
        app.world_mut().flush();

        let world = app.world();
        assert!(world.get::<Button>(button).is_some(), "must have Button");
        assert!(
            world.get::<Interaction>(button).is_some(),
            "Button requires Interaction",
        );
        assert!(
            world.get::<BackgroundColor>(button).is_some(),
            "must have BackgroundColor",
        );
        assert!(
            world.get::<UiBorderColor>(button).is_some(),
            "must have BorderColor",
        );
        assert_eq!(
            world.get::<Themed>(button).map(|t| **t),
            Some(ThemeRole::Button),
            "must carry Themed(Button)",
        );

        Ok(())
    }

    /// `spawn_panel` builds a `Themed(Panel)` node with background + border.
    #[test]
    fn spawned_panel_is_themed_with_visuals() -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;

        let panel = {
            let mut commands = app.world_mut().commands();
            spawn_panel(&mut commands, &theme_res)
        };
        app.world_mut().flush();

        let world = app.world();
        assert!(world.get::<Node>(panel).is_some(), "must have Node");
        assert!(
            world.get::<BackgroundColor>(panel).is_some(),
            "must have BackgroundColor",
        );
        assert!(
            world.get::<UiBorderColor>(panel).is_some(),
            "must have BorderColor",
        );
        assert_eq!(
            world.get::<Themed>(panel).map(|t| **t),
            Some(ThemeRole::Panel),
            "must carry Themed(Panel)",
        );

        Ok(())
    }

    /// The text child of a `spawn_button` carries the button sub-theme font handle,
    /// size, and text color, and is itself `Themed(ButtonText)`.
    #[test]
    fn spawned_button_text_child_is_themed_button_text_from_theme()
    -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;

        let button = {
            let mut commands = app.world_mut().commands();
            spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
        };
        app.world_mut().flush();

        let world = app.world();
        let children = world
            .get::<Children>(button)
            .map(|c| c.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        assert_eq!(children.len(), 1, "button must have exactly one text child");
        let child = children[0];

        assert_eq!(
            world.get::<Themed>(child).map(|t| **t),
            Some(ThemeRole::ButtonText),
            "text child must be Themed(ButtonText)",
        );
        assert_eq!(
            world.get::<TextFont>(child).map(|f| f.font.clone()),
            Some(Handle::<Font>::default()),
            "text child must carry the button font handle",
        );
        assert!(
            world
                .get::<TextFont>(child)
                .is_some_and(|f| (f.font_size - 18.0).abs() < f32::EPSILON),
            "text child must carry the button font size",
        );
        assert_eq!(
            world.get::<UiTextColor>(child).map(|c| c.0),
            Some(Color::srgb(0.84, 0.80, 0.73)),
            "text child must carry the button text color",
        );

        Ok(())
    }

    /// A `DisabledButton`'s background is the button sub-theme's explicit
    /// `disabled` fill after `paint_disabled_buttons` runs, and it stays `Themed`.
    #[test]
    fn disabled_button_is_painted_from_theme() -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
        app.insert_resource(theme_res.clone());
        app.add_systems(
            Update,
            (apply_theme, paint_disabled_buttons)
                .chain()
                .run_if(resource_exists::<GdtfTheme>),
        );

        let button = {
            let mut commands = app.world_mut().commands();
            spawn_button(
                &mut commands,
                &theme_res,
                ButtonLabel::new("Locked"),
                DisabledButton,
            )
        };
        app.world_mut().flush();

        app.update();

        let world = app.world();
        let expected = *theme_res.button.disabled;
        assert_eq!(
            world.get::<BackgroundColor>(button).map(|c| c.0),
            Some(expected),
            "disabled button must be the button sub-theme's explicit disabled fill",
        );
        assert!(
            world.get::<Themed>(button).is_some(),
            "disabled button must remain Themed so apply_theme still reaches it",
        );

        Ok(())
    }
}

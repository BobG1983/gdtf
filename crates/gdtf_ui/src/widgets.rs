//! Reusable, theme-seam widget builders and the disabled-button marker.
//!
//! This module owns the two spawn helpers the menu / HUD work builds its UI
//! from — [`spawn_panel`] and [`spawn_button`] — plus the [`DisabledButton`]
//! marker and the [`dim_disabled_buttons`] system that dims it.
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
//! ## Disabled buttons
//!
//! A [`DisabledButton`] is dimmed by [`dim_disabled_buttons`] (a theme-derived
//! reduction of the panel fill, re-applied after
//! [`apply_theme`](crate::themed::apply_theme)) and is skipped entirely by the
//! interaction layer (which filters `Without<DisabledButton>`). It stays
//! [`Themed`](crate::themed::Themed), so the base-look pass still reaches it.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use crate::{
    theme::{GdtfTheme, PanelBg},
    themed::{ThemeRole, Themed},
};

/// Marker tagging a button that is currently disabled.
///
/// A disabled button is **dimmed** ([`dim_disabled_buttons`] writes a
/// theme-derived, lowered-opacity fill over its base) and is **skipped** by the
/// interaction layer (the interaction system queries `Without<DisabledButton>`,
/// so a disabled button never hover/press-swaps). It remains
/// [`Themed`](crate::themed::Themed): the central base-look pass still re-paints
/// it, and the dim is composed on top of that fresh base.
///
/// A unit marker — it carries no data; presence alone is the signal
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisabledButton;

/// The opacity a disabled widget's theme-derived fill is multiplied down to.
///
/// A named newtype over the alpha scale factor rather than a bare `f32`
/// (no-bare-types rule): [`dim_disabled_buttons`] takes the live theme's
/// [`PanelBg`](crate::theme::PanelBg) and multiplies its alpha by this factor,
/// so the dim tracks the *current* theme fill instead of being a hardcoded
/// gray. [`DimFactor::DISABLED`] is the single value used for disabled buttons.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct DimFactor(f32);

impl DimFactor {
    /// The opacity factor a [`DisabledButton`]'s fill is dimmed to: 40% of the
    /// theme panel fill's alpha. Lifted enough to read as "present but inert".
    pub const DISABLED: Self = Self(0.4);
}

/// Derives the dimmed fill of a disabled widget from the live panel fill.
///
/// Takes the theme's [`PanelBg`](crate::theme::PanelBg) and scales its alpha by
/// `factor`, leaving the RGB untouched — so the dim is a theme-derived variant
/// of the *current* fill, never a hardcoded color. Returning a fresh
/// [`Color`](bevy::prelude::Color) keeps the transformation pure and testable.
#[must_use]
pub fn dimmed_fill(panel_bg: PanelBg, factor: DimFactor) -> Color {
    let base = panel_bg.to_srgba();
    Color::srgba(base.red, base.green, base.blue, base.alpha * *factor)
}

/// Spawns a theme-seam panel and returns its [`Entity`].
///
/// Builds a [`Node`](bevy::ui::Node) with a
/// [`BackgroundColor`](bevy::ui::BackgroundColor),
/// [`BorderColor`](bevy::ui::BorderColor), and
/// [`BorderRadius`](bevy::ui::BorderRadius)-bearing node, and attaches
/// [`Themed(ThemeRole::Panel)`](crate::themed::Themed). The initial colors,
/// border width ([`BorderWidthPx`](crate::theme::BorderWidthPx)), corner radius
/// ([`CornerRadiusPx`](crate::theme::CornerRadiusPx)), and content-margin
/// padding ([`ContentMargin`](crate::theme::ContentMargin)) are all read from
/// `theme` — never literals.
///
/// The colors written here are *initial*, not frozen:
/// [`apply_theme`](crate::themed::apply_theme) re-derives the identical look
/// from the live [`GdtfTheme`](crate::theme::GdtfTheme) every run, so re-running
/// it reproduces them and a hot-reload re-paints the panel.
pub fn spawn_panel(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    commands.spawn(panel_bundle(theme)).id()
}

/// Spawns a theme-seam button and returns its [`Entity`].
///
/// Builds a [`Button`](bevy::ui::Button) tree — a node with
/// [`Interaction`](bevy::ui::Interaction) (required by `Button`),
/// [`BackgroundColor`](bevy::ui::BackgroundColor),
/// [`BorderColor`](bevy::ui::BorderColor), and
/// [`BorderRadius`](bevy::ui::BorderRadius) — carrying a
/// [`Text`](bevy::prelude::Text) child with a
/// [`TextFont`](bevy::text::TextFont) (the theme's resolved
/// [`Handle<Font>`](bevy::prelude::Handle) at
/// [`FontSizePt`](crate::theme::FontSizePt)) and a
/// [`TextColor`](bevy::text::TextColor). It attaches
/// [`Themed(ThemeRole::Panel)`](crate::themed::Themed) and the caller-supplied
/// `marker` bundle.
///
/// `label` is the button caption; `marker` is any [`Bundle`] the caller wants on
/// the button (a focus/action marker, a [`DisabledButton`], etc.). All base
/// colors come from `theme`, not literals, and are re-derived by
/// [`apply_theme`](crate::themed::apply_theme) every run (the Themed seam).
pub fn spawn_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: ButtonLabel,
    marker: impl Bundle,
) -> Entity {
    let font = theme.font.clone();
    let font_size = *theme.font_size_pt;
    let text_color = *theme.text;

    commands
        .spawn((Button, panel_bundle(theme), marker))
        .with_children(|parent| {
            parent.spawn((
                Themed(ThemeRole::Text),
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

/// Builds the shared panel-look bundle (node + initial theme-derived colors +
/// the `Themed(Panel)` marker) used by both [`spawn_panel`] and the
/// [`spawn_button`] root.
///
/// Factored out so a panel and a button's root share one tree definition: a
/// button is a panel that additionally reacts to interaction. The colors are
/// initial-only — [`apply_theme`](crate::themed::apply_theme) re-derives them.
fn panel_bundle(theme: &GdtfTheme) -> impl Bundle {
    let node = Node {
        border: UiRect::all(Val::Px(*theme.border_width_px)),
        border_radius: BorderRadius::all(Val::Px(*theme.corner_radius_px)),
        padding: UiRect::px(
            *theme.content_margin.l,
            *theme.content_margin.r,
            *theme.content_margin.t,
            *theme.content_margin.b,
        ),
        ..default()
    };

    (
        Themed(ThemeRole::Panel),
        node,
        BackgroundColor(*theme.panel_bg),
        UiBorderColor::all(*theme.border_color),
    )
}

/// Dims every [`DisabledButton`] by writing a theme-derived, lowered-opacity
/// fill over its [`BackgroundColor`](bevy::ui::BackgroundColor).
///
/// Runs **after** [`apply_theme`](crate::themed::apply_theme) (the
/// [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set) so it
/// composes on top of the freshest base fill (bevy-traps rule 3); a hot-reload
/// therefore re-derives the dim from the new palette. The fill is
/// [`dimmed_fill`] of the live [`PanelBg`](crate::theme::PanelBg) — never a
/// hardcoded gray.
///
/// Takes the theme as `Option<Res<GdtfTheme>>` so it is inert (rather than
/// panicking) before the resource is populated (bevy-traps rule 1).
pub fn dim_disabled_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut disabled: Query<&mut BackgroundColor, With<DisabledButton>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let dim = dimmed_fill(theme.panel_bg, DimFactor::DISABLED);
    for mut background in &mut disabled {
        background.0 = dim;
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

    /// Builds a [`GdtfTheme`] through the real resolution path (deserialize a
    /// spec, then [`GdtfThemeSpec::resolve`]) with a defaulted font handle. The
    /// theme newtypes have private fields, so this respects encapsulation while
    /// exercising the genuine runtime resolution. Returns the `ron` error so a
    /// malformed literal surfaces via `?` rather than a denied `unwrap`/`panic`.
    fn theme(
        panel: [f32; 4],
        hover: [f32; 4],
        press: [f32; 4],
    ) -> Result<GdtfTheme, ron::error::SpannedError> {
        let ron = format!(
            "(\
             text: (0.84, 0.80, 0.73, 1.0), \
             panel_bg: ({pr}, {pg}, {pb}, {pa}), \
             border_color: (0.20, 0.20, 0.24, 1.0), \
             border_width_px: 1.0, \
             corner_radius_px: 2.0, \
             margin_left_px: 8.0, margin_right_px: 8.0, \
             margin_top_px: 6.0, margin_bottom_px: 6.0, \
             font_size_pt: 18.0, \
             font_key: \"fonts/test.ttf\", \
             hover_bg: ({hr}, {hg}, {hb}, {ha}), \
             press_bg: ({sr}, {sg}, {sb}, {sa}))",
            pr = panel[0],
            pg = panel[1],
            pb = panel[2],
            pa = panel[3],
            hr = hover[0],
            hg = hover[1],
            hb = hover[2],
            ha = hover[3],
            sr = press[0],
            sg = press[1],
            sb = press[2],
            sa = press[3],
        );
        let spec: GdtfThemeSpec = ron::from_str(&ron)?;
        Ok(spec.resolve(Handle::<Font>::default()))
    }

    /// `spawn_button` builds a button carrying the interaction plumbing
    /// (`Button` + `Interaction`), the panel visuals (`BackgroundColor` +
    /// `BorderColor`), and the `Themed(Panel)` marker — AC#2.
    ///
    /// Pin-discriminating: dropping any of those components, or the Themed
    /// marker, fails an assert.
    #[test]
    fn spawned_button_has_interaction_visuals_and_themed_marker()
    -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        app.insert_resource(theme(
            [0.08, 0.08, 0.10, 1.0],
            [0.20, 0.20, 0.24, 1.0],
            [0.04, 0.04, 0.06, 1.0],
        )?);

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
            Some(ThemeRole::Panel),
            "must carry Themed(Panel)",
        );

        Ok(())
    }

    /// `spawn_panel` builds a `Themed(Panel)` node with background + border —
    /// AC#1.
    #[test]
    fn spawned_panel_is_themed_with_visuals() -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        let theme_res = theme(
            [0.08, 0.08, 0.10, 1.0],
            [0.20, 0.20, 0.24, 1.0],
            [0.04, 0.04, 0.06, 1.0],
        )?;

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

    /// The text child of a `spawn_button` carries the theme font handle, size,
    /// and text color, and is itself `Themed(Text)` — AC#2.
    #[test]
    fn spawned_button_text_child_is_themed_text_from_theme() -> Result<(), ron::error::SpannedError>
    {
        let mut app = App::new();
        let theme_res = theme(
            [0.08, 0.08, 0.10, 1.0],
            [0.20, 0.20, 0.24, 1.0],
            [0.04, 0.04, 0.06, 1.0],
        )?;

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
            Some(ThemeRole::Text),
            "text child must be Themed(Text)",
        );
        assert_eq!(
            world.get::<TextFont>(child).map(|f| f.font.clone()),
            Some(Handle::<Font>::default()),
            "text child must carry the theme font handle",
        );
        assert!(
            world
                .get::<TextFont>(child)
                .is_some_and(|f| (f.font_size - 18.0).abs() < f32::EPSILON),
            "text child must carry the theme font size",
        );
        assert_eq!(
            world.get::<UiTextColor>(child).map(|c| c.0),
            Some(Color::srgb(0.84, 0.80, 0.73)),
            "text child must carry the theme text color",
        );

        Ok(())
    }

    /// A `DisabledButton`'s background is the theme-derived dim (lowered-alpha
    /// panel fill) after `dim_disabled_buttons` runs, and it stays `Themed` —
    /// AC#5.
    #[test]
    fn disabled_button_is_dimmed_from_theme() -> Result<(), ron::error::SpannedError> {
        let mut app = App::new();
        let theme_res = theme(
            [0.08, 0.08, 0.10, 1.0],
            [0.20, 0.20, 0.24, 1.0],
            [0.04, 0.04, 0.06, 1.0],
        )?;
        app.insert_resource(theme_res.clone());
        app.add_systems(
            Update,
            (apply_theme, dim_disabled_buttons)
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
        let expected = dimmed_fill(theme_res.panel_bg, DimFactor::DISABLED);
        assert_eq!(
            world.get::<BackgroundColor>(button).map(|c| c.0),
            Some(expected),
            "disabled button must be the theme-derived dim",
        );
        assert!(
            world.get::<Themed>(button).is_some(),
            "disabled button must remain Themed so apply_theme still reaches it",
        );

        Ok(())
    }
}

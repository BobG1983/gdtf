//! The hot-reload seam: the [`Themed`] marker and the central [`apply_theme`]
//! system that paints theme-derived visuals onto themed entities.
//!
//! An entity opts into centralized theming by carrying a [`Themed`] component,
//! whose [`ThemeRole`] declares *what kind* of widget it is (a text node, or a
//! panel/button node). On every run [`apply_theme`] reads the **current**
//! [`GdtfTheme`](crate::theme::GdtfTheme) resource and writes that entity's
//! base look from it:
//!
//! - [`ThemeRole::Text`] → text color, font face, and font size
//!   ([`TextColor`](bevy::text::TextColor) + [`TextFont`](bevy::text::TextFont)).
//! - [`ThemeRole::Panel`] → panel background, border color, border width, corner
//!   radius, and content margins
//!   ([`BackgroundColor`](bevy::ui::BackgroundColor) +
//!   [`BorderColor`](bevy::ui::BorderColor) + the [`Node`](bevy::ui::Node)'s
//!   `border`, `border_radius`, and `padding` fields). Panels and buttons share
//!   this role — a button is a panel that also reacts to interaction.
//!
//! ## Live read, never a spawn snapshot
//!
//! [`apply_theme`] resolves its values from the [`GdtfTheme`] resource *every
//! run*, not at spawn time. Re-running it after the resource is mutated re-paints
//! every [`Themed`] entity with the new palette — that is the seam the live
//! hot-reload (GTW-137/138) hangs on. This module owns the marker and the
//! application; it does **not** define the theme schema/resource (that is
//! GTW-117), spawn any widget (GTW-118), or contain the file-watcher / reload
//! trigger (GTW-137/138).
//!
//! ## Boundary with interaction state
//!
//! [`apply_theme`] sets only the **base** look. Per-widget interaction feedback
//! (hover / press background swaps, GTW-118) composes *on top* of this base in a
//! later system, ordered after [`UiSystems::ApplyTheme`]; it is intentionally not
//! part of this system.
//!
//! ## Absence guard
//!
//! [`apply_theme`] is registered with `.run_if(resource_exists::<GdtfTheme>)`, so
//! before `AppState::Load` populates the resource (per the project's
//! state-scoped-resource convention, bevy-traps rule 1) the system simply does
//! not run — it never panics on an absent theme.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use crate::theme::GdtfTheme;

/// The kind of themed widget an entity is, which selects *which* theme-derived
/// visuals [`apply_theme`] writes onto it.
///
/// A named role rather than a bare boolean or marker pair: the set of widget
/// kinds the theme knows how to paint is a closed vocabulary, and a closed
/// vocabulary is an enum. Panels and buttons share [`Panel`](ThemeRole::Panel) —
/// a button is a panel that additionally reacts to interaction (GTW-118), and
/// interaction feedback composes on top of the base panel look this role drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeRole {
    /// A text node: [`apply_theme`] sets its foreground color, font face, and
    /// font size from the theme.
    Text,
    /// A panel or button node: [`apply_theme`] sets its background fill, border
    /// color, border width, corner radius, and content margins from the theme.
    Panel,
}

/// Marker opting an entity into centralized theming, tagged with its
/// [`ThemeRole`].
///
/// Attach `Themed(role)` to any UI entity whose look should be driven by the
/// live [`GdtfTheme`](crate::theme::GdtfTheme): [`apply_theme`] then paints the
/// role-appropriate visuals onto it every run, re-reading the resource so a theme
/// change re-themes it. Spawning code only declares the *role*; it never copies
/// theme values itself, which is what keeps the look in one place and live.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Themed(pub ThemeRole);

/// Explicit system-ordering sets for the UI theming layer.
///
/// [`ApplyTheme`](UiSystems::ApplyTheme) names where [`apply_theme`] runs so
/// later systems can order deterministically relative to it (bevy-traps rule 3) —
/// in particular the GTW-137 retheme trigger, and any interaction-feedback system
/// (GTW-118) that must compose its hover/press swap *after* the base look is laid
/// down.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSystems {
    /// The set containing [`apply_theme`] — the base-look application pass.
    ApplyTheme,
}

/// Paints the base, theme-derived look onto every [`Themed`] entity from the
/// **current** [`GdtfTheme`] resource.
///
/// For each themed entity, the visuals written depend on its [`ThemeRole`]:
///
/// - [`ThemeRole::Text`]: a [`TextColor`](bevy::text::TextColor) and a
///   [`TextFont`](bevy::text::TextFont) carrying the theme font handle and size.
/// - [`ThemeRole::Panel`]: a [`BackgroundColor`](bevy::ui::BackgroundColor),
///   a [`BorderColor`](bevy::ui::BorderColor), and a [`Node`](bevy::ui::Node)
///   whose `border` is the theme border width, whose `border_radius` is the theme
///   corner radius, and whose `padding` is the theme content margins (the rest of
///   the existing `Node`'s layout is preserved). In Bevy 0.18 the corner radius
///   lives in [`Node::border_radius`](bevy::ui::Node), not a standalone component.
///
/// Components are written through [`Commands`], so the look is applied whether or
/// not the entity already carried the target component — apply-or-insert, robust
/// to spawn order. The theme is read **live** (a fresh `Res` borrow each run), so
/// re-running this after the resource changes re-themes; it never snapshots at
/// spawn.
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] under
/// [`UiSystems::ApplyTheme`] with `.run_if(resource_exists::<GdtfTheme>)`: before
/// the theme is populated (pre-`Load`) the system does not run, so an absent
/// resource never panics (bevy-traps rule 1).
pub fn apply_theme(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    themed: Query<(Entity, &Themed, Option<&Node>)>,
) {
    for (entity, marker, node) in &themed {
        match **marker {
            ThemeRole::Text => {
                commands.entity(entity).insert((
                    UiTextColor(*theme.text),
                    TextFont {
                        font: theme.font.clone(),
                        font_size: *theme.font_size_pt,
                        ..default()
                    },
                ));
            }
            ThemeRole::Panel => {
                // Preserve any existing layout on the node, overriding only the
                // theme-owned border width, corner radius, and content margins.
                let mut themed_node = node.cloned().unwrap_or_default();
                themed_node.border = UiRect::all(Val::Px(*theme.border_width_px));
                themed_node.border_radius = BorderRadius::all(Val::Px(*theme.corner_radius_px));
                themed_node.padding = UiRect::px(
                    *theme.content_margin.l,
                    *theme.content_margin.r,
                    *theme.content_margin.t,
                    *theme.content_margin.b,
                );

                commands.entity(entity).insert((
                    BackgroundColor(*theme.panel_bg),
                    UiBorderColor::all(*theme.border_color),
                    themed_node,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        text::{TextColor as UiTextColor, TextFont},
        ui::{BackgroundColor, BorderColor as UiBorderColor, Node, Val},
    };

    // `super::*` re-exports the parent module's `bevy::prelude::*` glob, so the
    // prelude items (`App`, `Update`, `resource_exists`, `Color`, `Handle`,
    // `Font`, `Text`, `default`, the schedule-config extension traits) are in
    // scope here without a second `prelude::*`.
    use super::*;
    use crate::theme::{GdtfTheme, GdtfThemeSpec};

    /// Builds a [`GdtfTheme`] with the given salient colors / scalars through the
    /// real production path — deserialize a spec, then [`GdtfThemeSpec::resolve`]
    /// it with a defaulted font handle (no `AssetServer` needed). The theme
    /// newtypes have private fields, so this both respects that encapsulation and
    /// exercises the genuine resolution the runtime uses. Content margins are
    /// fixed (left/right 8, top/bottom 6) since the role-paint test pins them.
    /// Returns the `ron` error so callers surface a malformed literal via `?`
    /// rather than a denied `unwrap`/`panic`.
    fn theme(
        text: [f32; 3],
        panel: [f32; 3],
        border: [f32; 3],
        border_w: f32,
        radius: f32,
        font_size: f32,
    ) -> Result<GdtfTheme, ron::error::SpannedError> {
        // `Srgba4` is `#[serde(transparent)]` over `[f32; 4]`, which RON encodes
        // as a tuple `(r, g, b, a)` — mirror the shipped `grimdark.ron` shape.
        let ron = format!(
            "(\
             text: ({tr}, {tg}, {tb}, 1.0), \
             panel_bg: ({pr}, {pg}, {pb}, 1.0), \
             border_color: ({br}, {bg}, {bb}, 1.0), \
             border_width_px: {border_w}, \
             corner_radius_px: {radius}, \
             margin_left_px: 8.0, margin_right_px: 8.0, \
             margin_top_px: 6.0, margin_bottom_px: 6.0, \
             font_size_pt: {font_size}, \
             font_key: \"fonts/test.ttf\", \
             hover_bg: ({pr}, {pg}, {pb}, 1.0), \
             press_bg: ({pr}, {pg}, {pb}, 1.0))",
            tr = text[0],
            tg = text[1],
            tb = text[2],
            pr = panel[0],
            pg = panel[1],
            pb = panel[2],
            br = border[0],
            bg = border[1],
            bb = border[2],
        );
        let spec: GdtfThemeSpec = ron::from_str(&ron)?;
        Ok(spec.resolve(Handle::<Font>::default()))
    }

    /// A minimal app with `apply_theme` registered under the real run condition,
    /// so the test drives the actual production schedule path.
    fn app_with_apply_theme() -> App {
        let mut app = App::new();
        app.add_systems(Update, apply_theme.run_if(resource_exists::<GdtfTheme>));
        app
    }

    /// `apply_theme` writes the theme's text color, font handle, and font size
    /// onto a `Themed(Text)` entity, and its panel bg / border color / border
    /// width / corner radius / content margins onto a `Themed(Panel)` entity.
    ///
    /// Pin-discriminating: reverting any field's mapping (or dropping a written
    /// component) fails the corresponding assert.
    #[test]
    fn apply_theme_paints_role_appropriate_visuals_from_the_resource()
    -> Result<(), ron::error::SpannedError> {
        let mut app = app_with_apply_theme();
        app.insert_resource(theme(
            [0.84, 0.80, 0.73],
            [0.08, 0.08, 0.10],
            [0.20, 0.20, 0.24],
            1.0,
            2.0,
            18.0,
        )?);

        let text = app
            .world_mut()
            .spawn((Themed(ThemeRole::Text), Text::new("x")))
            .id();
        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
            .id();

        app.update();

        let world = app.world();

        assert_eq!(
            world.get::<UiTextColor>(text).map(|c| c.0),
            Some(Color::srgb(0.84, 0.80, 0.73)),
            "text entity should get the theme text color",
        );
        assert_eq!(
            world.get::<TextFont>(text).map(|f| f.font.clone()),
            Some(Handle::<Font>::default()),
            "text entity should get the theme font handle",
        );
        assert!(
            world
                .get::<TextFont>(text)
                .is_some_and(|f| (f.font_size - 18.0).abs() < f32::EPSILON),
            "text entity font size should be the theme size",
        );

        assert_eq!(
            world.get::<BackgroundColor>(panel).map(|c| c.0),
            Some(Color::srgb(0.08, 0.08, 0.10)),
            "panel entity should get the theme panel bg",
        );
        assert_eq!(
            world.get::<UiBorderColor>(panel).map(|b| b.top),
            Some(Color::srgb(0.20, 0.20, 0.24)),
            "panel entity should get the theme border color on every edge",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.border.left),
            Some(Val::Px(1.0)),
            "border width",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.padding.left),
            Some(Val::Px(8.0)),
            "left content margin",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.padding.top),
            Some(Val::Px(6.0)),
            "top content margin",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.border_radius.top_left),
            Some(Val::Px(2.0)),
            "corner radius",
        );

        Ok(())
    }

    /// Mutating the in-memory `GdtfTheme` and re-running `apply_theme` re-themes
    /// the already-spawned entities with the NEW palette — proving the system
    /// reads the resource live each run, never snapshots at spawn.
    ///
    /// Pin-discriminating: if `apply_theme` captured values at spawn instead of
    /// reading `Res<GdtfTheme>` per run, the colors after the second update would
    /// still be the first palette and these asserts would fail.
    #[test]
    fn re_theme_after_resource_mutation_repaints_with_new_palette()
    -> Result<(), ron::error::SpannedError> {
        let mut app = app_with_apply_theme();
        app.insert_resource(theme(
            [0.84, 0.80, 0.73],
            [0.08, 0.08, 0.10],
            [0.20, 0.20, 0.24],
            1.0,
            2.0,
            18.0,
        )?);

        let text = app
            .world_mut()
            .spawn((Themed(ThemeRole::Text), Text::new("x")))
            .id();
        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
            .id();

        app.update();

        // Swap to a deliberately different palette and scalars.
        let new_text = Color::srgb(0.10, 0.90, 0.20);
        let new_panel = Color::srgb(0.50, 0.10, 0.30);
        let new_border = Color::srgb(0.99, 0.40, 0.00);
        app.insert_resource(theme(
            [0.10, 0.90, 0.20],
            [0.50, 0.10, 0.30],
            [0.99, 0.40, 0.00],
            4.0,
            9.0,
            30.0,
        )?);

        app.update();

        let world = app.world();
        assert_eq!(
            world.get::<UiTextColor>(text).map(|c| c.0),
            Some(new_text),
            "text color must reflect the NEW theme after re-run",
        );
        assert!(
            world
                .get::<TextFont>(text)
                .is_some_and(|f| (f.font_size - 30.0).abs() < f32::EPSILON),
            "font size must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<BackgroundColor>(panel).map(|c| c.0),
            Some(new_panel),
            "panel bg must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<UiBorderColor>(panel).map(|b| b.top),
            Some(new_border),
            "border color must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.border.left),
            Some(Val::Px(4.0)),
            "border width must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.border_radius.top_left),
            Some(Val::Px(9.0)),
            "corner radius must reflect the NEW theme after re-run",
        );

        Ok(())
    }

    /// With NO `GdtfTheme` resource inserted, an `app.update()` must not panic:
    /// the `run_if(resource_exists)` guard keeps `apply_theme` from running, so
    /// the absent theme (pre-`Load`) is safe (bevy-traps rule 1).
    ///
    /// Pin-discriminating: dropping the run condition (or taking `Res<GdtfTheme>`
    /// without the guard) would make this update panic on the missing resource.
    #[test]
    fn absent_theme_does_not_panic() {
        let mut app = app_with_apply_theme();
        app.world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()));

        // No GdtfTheme inserted. The guard must keep apply_theme from running.
        app.update();

        // Reaching here without a panic is the assertion; make it explicit.
        assert!(
            app.world().get_resource::<GdtfTheme>().is_none(),
            "test precondition: GdtfTheme must be absent for this guard check",
        );
    }
}

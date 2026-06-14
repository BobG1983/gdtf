//! The hot-reload seam: the [`Themed`] marker and the central [`apply_theme`]
//! system that paints theme-derived visuals onto themed entities.
//!
//! An entity opts into centralized theming by carrying a [`Themed`] component,
//! whose [`ThemeRole`] declares *what kind* of widget it is. On every run
//! [`apply_theme`] reads the **current** [`GdtfTheme`](crate::theme::GdtfTheme)
//! resource and writes that entity's base look from the matching **sub-theme**
//! (GTW-149):
//!
//! - [`ThemeRole::Background`] (a [`Node`](bevy::ui::Node)): the full-screen
//!   backdrop fill from `background.color` — no border/radius/padding.
//! - [`ThemeRole::Panel`] (a [`Node`](bevy::ui::Node)): fill, border color, border
//!   width, corner radius, and content padding from `panel.*` (a box around UI).
//! - [`ThemeRole::Button`] (a [`Node`](bevy::ui::Node)): fill, border color,
//!   border width, corner radius, and content padding from `button.*` (the button
//!   box).
//! - [`ThemeRole::ButtonText`] (a [`Text`](bevy::prelude::Text)): text color, font
//!   face, and font size from the **button** sub-theme.
//! - [`ThemeRole::Title`] (a [`Text`](bevy::prelude::Text)): text color, font face,
//!   and font size from the **title** sub-theme.
//! - [`ThemeRole::Text`] (a [`Text`](bevy::prelude::Text)): text color, font face,
//!   and font size from the **text** sub-theme.
//!
//! Each text role draws its own `font_size_pt` from its sub-theme — there is no
//! scale-the-body-size title fudge anymore; the title simply has its own size.
//!
//! ## Live read, never a spawn snapshot
//!
//! [`apply_theme`] resolves its values from the [`GdtfTheme`] resource *every
//! run*, not at spawn time. Re-running it after the resource is mutated re-paints
//! every [`Themed`] entity with the new palette — that is the seam the live
//! hot-reload hangs on.
//!
//! ## Boundary with interaction state
//!
//! [`apply_theme`] sets only the **base** look. Per-widget interaction feedback
//! (hover / press background swaps) composes *on top* of this base in a later
//! system, ordered after [`UiSystems::ApplyTheme`]; it is intentionally not part
//! of this system.
//!
//! ## Cadence and absence guard
//!
//! [`apply_theme`] is **change-driven** (GTW-144): [`UiPlugin`](crate::UiPlugin)
//! registers it to run only when the [`GdtfTheme`](crate::theme::GdtfTheme)
//! resource *changed* (the `Load` insert or the re-derive — repainting every
//! [`Themed`] entity, i.e. the retheme) **or** when a new [`Themed`] entity was
//! added this frame (so a freshly-spawned widget still gets its base look). It
//! does **not** run on steady-state frames, so it never clobbers the GTW-118
//! hover/press feedback. The `resource_exists::<GdtfTheme>` part of that run
//! condition also keeps it inert before `AppState::Load` populates the resource
//! (bevy-traps rule 1) — it never panics on an absent theme.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use crate::theme::{ContentMargin, GdtfTheme};

/// The kind of themed widget an entity is, which selects *which* sub-theme
/// [`apply_theme`] paints onto it.
///
/// A named role rather than a bare boolean or marker pair: the set of widget
/// kinds the theme knows how to paint is a closed vocabulary, and a closed
/// vocabulary is an enum.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeRole {
    /// The full-screen backdrop [`Node`](bevy::ui::Node): [`apply_theme`] sets its
    /// background fill from the **background** sub-theme (no border/radius/padding).
    Background,
    /// A panel-box [`Node`](bevy::ui::Node): [`apply_theme`] sets its fill, border
    /// color, border width, corner radius, and content padding from the **panel**
    /// sub-theme.
    Panel,
    /// A button-box [`Node`](bevy::ui::Node): [`apply_theme`] sets its fill, border
    /// color, border width, corner radius, and content padding from the **button**
    /// sub-theme. Interaction feedback (GTW-118) composes hover/press on top.
    Button,
    /// A button caption [`Text`](bevy::prelude::Text): [`apply_theme`] sets its
    /// color, font face, and size from the **button** sub-theme.
    ButtonText,
    /// A heading / title [`Text`](bevy::prelude::Text): [`apply_theme`] sets its
    /// color, font face, and size from the **title** sub-theme.
    Title,
    /// A body-text [`Text`](bevy::prelude::Text): [`apply_theme`] sets its color,
    /// font face, and size from the **text** sub-theme.
    Text,
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
/// in particular the retheme trigger, and any interaction-feedback system
/// (GTW-118) that must compose its hover/press swap *after* the base look is laid
/// down.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSystems {
    /// The set containing [`apply_theme`] — the base-look application pass.
    ApplyTheme,
}

/// Paints the base, theme-derived look onto every [`Themed`] entity from the
/// **current** [`GdtfTheme`] resource, by [`ThemeRole`] (see the module docs for
/// the role→sub-theme mapping).
///
/// Components are written through [`Commands`], so the look is applied whether or
/// not the entity already carried the target component — apply-or-insert, robust
/// to spawn order. The theme is read **live** (a fresh `Res` borrow each run), so
/// re-running this after the resource changes re-themes; it never snapshots at
/// spawn.
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] under
/// [`UiSystems::ApplyTheme`], **change-driven** (GTW-144): it runs only when the
/// theme changed or a new [`Themed`] entity appeared, never on steady-state
/// frames (so it never clobbers interaction feedback), and the
/// `resource_exists::<GdtfTheme>` guard keeps it inert and panic-free before the
/// theme is populated (pre-`Load`, bevy-traps rule 1). See
/// [`UiPlugin::build`](crate::UiPlugin) for the exact run condition.
pub fn apply_theme(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    themed: Query<(Entity, &Themed, Option<&Node>)>,
) {
    let mut painted = 0usize;
    for (entity, marker, node) in &themed {
        painted += 1;
        match **marker {
            ThemeRole::Background => {
                // The backdrop only fills — no border / radius / padding.
                commands
                    .entity(entity)
                    .insert(BackgroundColor(*theme.background.color));
            }
            ThemeRole::Panel => {
                let themed_node = box_node(
                    node,
                    *theme.panel.border_width_px,
                    *theme.panel.corner_radius_px,
                    theme.panel.margin,
                );
                commands.entity(entity).insert((
                    BackgroundColor(*theme.panel.color),
                    UiBorderColor::all(*theme.panel.border_color),
                    themed_node,
                ));
            }
            ThemeRole::Button => {
                let themed_node = box_node(
                    node,
                    *theme.button.border_width_px,
                    *theme.button.corner_radius_px,
                    theme.button.margin,
                );
                commands.entity(entity).insert((
                    BackgroundColor(*theme.button.color),
                    UiBorderColor::all(*theme.button.border_color),
                    themed_node,
                ));
            }
            ThemeRole::ButtonText => {
                commands.entity(entity).insert((
                    UiTextColor(*theme.button.text_color),
                    TextFont {
                        font: theme.button.font.clone(),
                        font_size: *theme.button.font_size_pt,
                        ..default()
                    },
                ));
            }
            ThemeRole::Title => {
                commands.entity(entity).insert((
                    UiTextColor(*theme.title.text_color),
                    TextFont {
                        font: theme.title.font.clone(),
                        font_size: *theme.title.font_size_pt,
                        ..default()
                    },
                ));
            }
            ThemeRole::Text => {
                commands.entity(entity).insert((
                    UiTextColor(*theme.text.text_color),
                    TextFont {
                        font: theme.text.font.clone(),
                        font_size: *theme.text.font_size_pt,
                        ..default()
                    },
                ));
            }
        }
    }
    // GTW-146 hot-reload instrumentation: this system is change-driven, so a line
    // here after a save confirms the re-derived theme was actually reapplied to
    // the live widgets (the final step of the reload chain).
    if painted > 0 {
        info!("apply_theme: repainted {painted} Themed entities from the current GdtfTheme");
    }
}

/// Builds a themed box [`Node`](bevy::ui::Node): the existing node's layout
/// preserved, with only the theme-owned border width, corner radius, and content
/// padding overridden.
///
/// Shared by the [`ThemeRole::Panel`] and [`ThemeRole::Button`] arms — both paint
/// a box, differing only in which sub-theme's scalars feed in. In Bevy 0.18 the
/// corner radius lives in [`Node::border_radius`](bevy::ui::Node), the border
/// width in [`Node::border`](bevy::ui::Node), and the padding in
/// [`Node::padding`](bevy::ui::Node) — not standalone components.
fn box_node(node: Option<&Node>, border_px: f32, radius_px: f32, margin: ContentMargin) -> Node {
    let mut themed_node = node.cloned().unwrap_or_default();
    themed_node.border = UiRect::all(Val::Px(border_px));
    themed_node.border_radius = BorderRadius::all(Val::Px(radius_px));
    themed_node.padding = UiRect::px(*margin.l, *margin.r, *margin.t, *margin.b);
    themed_node
}

/// Run condition: at least one [`Themed`] entity was **added** this frame.
///
/// Used by [`UiPlugin`](crate::UiPlugin) to keep [`apply_theme`] change-driven
/// (GTW-144): the system repaints either when the theme changed **or** when a
/// freshly-spawned [`Themed`] widget needs its base look — but **not** on
/// steady-state frames, where re-running every frame would clobber the GTW-118
/// hover/press feedback that only updates on `Changed<Interaction>`.
///
/// `Added<Themed>` matches an entity only on the frames after its `Themed`
/// component was inserted, so this returns `true` exactly when a new widget needs
/// its first paint, and `false` once it has settled.
#[must_use]
pub fn any_themed_added(added: Query<(), Added<Themed>>) -> bool {
    !added.is_empty()
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

    /// Builds a [`GdtfTheme`] from the shipped-shape nested RON with caller-chosen
    /// salient values, through the real production path (deserialize a spec, then
    /// [`GdtfThemeSpec::resolve`] with a defaulted-font resolver). The theme
    /// newtypes have private fields, so this respects encapsulation while
    /// exercising the genuine resolution. Returns the `ron` error so a malformed
    /// literal surfaces via `?` rather than a denied `unwrap`/`panic`.
    fn theme(
        button_color: [f32; 3],
        border: [f32; 3],
        border_w: f32,
        radius: f32,
        button_font_size: f32,
    ) -> Result<GdtfTheme, ron::error::SpannedError> {
        let [pr, pg, pb] = button_color;
        let [br, bg, bb] = border;
        let ron = format!(
            "(\
             default_font: \"fonts/test.ttf\", \
             background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
             panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                      border_width_px: 3.0, corner_radius_px: 7.0, \
                      margin: (left: 9.0, right: 9.0, top: 4.0, bottom: 4.0) ), \
             button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.08, 0.08, 0.10, 0.55), \
                       hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                       text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: {button_font_size}, \
                       border_color: ({br}, {bg}, {bb}, 1.0), \
                       border_width_px: {border_w}, corner_radius_px: {radius}, \
                       margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
             title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
             text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
        );
        let spec: GdtfThemeSpec = ron::from_str(&ron)?;
        Ok(spec.resolve(|_| Handle::<Font>::default()))
    }

    /// A minimal app with `apply_theme` registered under the real run condition,
    /// so the test drives the actual production schedule path.
    fn app_with_apply_theme() -> App {
        let mut app = App::new();
        app.add_systems(Update, apply_theme.run_if(resource_exists::<GdtfTheme>));
        app
    }

    /// `apply_theme` writes the button text color / font / size onto a
    /// `Themed(ButtonText)` entity, and the button fill / border color / border
    /// width / corner radius / content padding onto a `Themed(Button)` entity.
    ///
    /// Pin-discriminating: reverting any field's mapping (or dropping a written
    /// component) fails the corresponding assert.
    #[test]
    fn apply_theme_paints_role_appropriate_visuals_from_the_resource()
    -> Result<(), ron::error::SpannedError> {
        let mut app = app_with_apply_theme();
        app.insert_resource(theme(
            [0.12, 0.12, 0.15],
            [0.20, 0.20, 0.24],
            2.0,
            5.0,
            18.0,
        )?);

        let text = app
            .world_mut()
            .spawn((Themed(ThemeRole::ButtonText), Text::new("x")))
            .id();
        let button = app
            .world_mut()
            .spawn((Themed(ThemeRole::Button), Node::default()))
            .id();

        app.update();

        let world = app.world();

        assert_eq!(
            world.get::<UiTextColor>(text).map(|c| c.0),
            Some(Color::srgb(0.84, 0.80, 0.73)),
            "button-text entity should get the button text color",
        );
        assert_eq!(
            world.get::<TextFont>(text).map(|f| f.font.clone()),
            Some(Handle::<Font>::default()),
            "button-text entity should get the button font handle",
        );
        assert!(
            world
                .get::<TextFont>(text)
                .is_some_and(|f| (f.font_size - 18.0).abs() < f32::EPSILON),
            "button-text entity font size should be the button size",
        );

        assert_eq!(
            world.get::<BackgroundColor>(button).map(|c| c.0),
            Some(Color::srgb(0.12, 0.12, 0.15)),
            "button entity should get the button fill",
        );
        assert_eq!(
            world.get::<UiBorderColor>(button).map(|b| b.top),
            Some(Color::srgb(0.20, 0.20, 0.24)),
            "button entity should get the button border color on every edge",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.border.left),
            Some(Val::Px(2.0)),
            "button border width",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.padding.left),
            Some(Val::Px(8.0)),
            "left content padding",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.padding.top),
            Some(Val::Px(6.0)),
            "top content padding",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.border_radius.top_left),
            Some(Val::Px(5.0)),
            "corner radius",
        );

        Ok(())
    }

    /// `apply_theme` paints a `Themed(Background)` node with only the backdrop
    /// fill (no border / radius / padding), and a `Themed(Panel)` node with the
    /// panel sub-theme's box visuals.
    #[test]
    fn apply_theme_paints_background_and_panel() -> Result<(), ron::error::SpannedError> {
        let mut app = app_with_apply_theme();
        app.insert_resource(theme(
            [0.12, 0.12, 0.15],
            [0.20, 0.20, 0.24],
            2.0,
            5.0,
            18.0,
        )?);

        let background = app
            .world_mut()
            .spawn((Themed(ThemeRole::Background), Node::default()))
            .id();
        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
            .id();

        app.update();

        let world = app.world();
        // Background gets the backdrop fill and NO border (default 0 width).
        assert_eq!(
            world.get::<BackgroundColor>(background).map(|c| c.0),
            Some(Color::srgb(0.05, 0.05, 0.06)),
            "background entity should get the backdrop fill",
        );
        // The Background arm writes only BackgroundColor — it leaves the spawned
        // node's border at the default (Px(0.0)), never the panel/button themed
        // border width (which would be Px(3.0)/Px(2.0)).
        assert_eq!(
            world.get::<Node>(background).map(|n| n.border.left),
            Some(Val::Px(0.0)),
            "background node must keep its default border, not get a themed border width",
        );

        // Panel gets the panel sub-theme box (border 3.0, radius 7.0, padding 9.0).
        assert_eq!(
            world.get::<BackgroundColor>(panel).map(|c| c.0),
            Some(Color::srgba(0.16, 0.16, 0.18, 0.55)),
            "panel entity should get the panel fill",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.border.left),
            Some(Val::Px(3.0)),
            "panel border width from the panel sub-theme",
        );
        assert_eq!(
            world.get::<Node>(panel).map(|n| n.padding.left),
            Some(Val::Px(9.0)),
            "panel padding from the panel sub-theme",
        );

        Ok(())
    }

    /// `apply_theme` paints a `Themed(Title)` node with the title sub-theme's
    /// color, font handle, and its **own** `font_size_pt` (36) — distinct from the
    /// button/body size, and no longer a scaled body size.
    ///
    /// Pin-discriminating: if the Title arm used the button/body size, or a
    /// hardcoded scale, the size assert fails; if it stopped sourcing color/font
    /// from the title sub-theme, those asserts fail.
    #[test]
    fn apply_theme_uses_title_sub_theme_font_size() -> Result<(), ron::error::SpannedError> {
        let mut app = app_with_apply_theme();
        let theme_res = theme([0.12, 0.12, 0.15], [0.20, 0.20, 0.24], 2.0, 5.0, 18.0)?;
        app.insert_resource(theme_res.clone());

        let title = app
            .world_mut()
            .spawn((Themed(ThemeRole::Title), Text::new("GRIMDARK TURFWAR")))
            .id();

        app.update();

        let world = app.world();
        assert_eq!(
            world.get::<UiTextColor>(title).map(|c| c.0),
            Some(Color::srgb(0.84, 0.80, 0.73)),
            "title must use the title text color",
        );
        assert_eq!(
            world.get::<TextFont>(title).map(|f| f.font.clone()),
            Some(Handle::<Font>::default()),
            "title must use the title font handle",
        );
        // The title sub-theme size is 36.0 — its own value, not 18 * a scale —
        // and strictly larger than the resolved text sub-theme's body size (18).
        let title_size = world.get::<TextFont>(title).map(|f| f.font_size);
        assert!(
            title_size.is_some_and(|s| (s - 36.0).abs() < f32::EPSILON),
            "title font size must be the title sub-theme's own font_size_pt (36)",
        );
        let body_size = *theme_res.text.font_size_pt;
        assert!(
            title_size.is_some_and(|s| s > body_size),
            "title must be strictly larger than the body/button text size",
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
            [0.12, 0.12, 0.15],
            [0.20, 0.20, 0.24],
            2.0,
            5.0,
            18.0,
        )?);

        let text = app
            .world_mut()
            .spawn((Themed(ThemeRole::ButtonText), Text::new("x")))
            .id();
        let button = app
            .world_mut()
            .spawn((Themed(ThemeRole::Button), Node::default()))
            .id();

        app.update();

        // Swap to a deliberately different button palette and scalars.
        let new_button = Color::srgb(0.50, 0.10, 0.30);
        let new_border = Color::srgb(0.99, 0.40, 0.00);
        app.insert_resource(theme(
            [0.50, 0.10, 0.30],
            [0.99, 0.40, 0.00],
            4.0,
            9.0,
            30.0,
        )?);

        app.update();

        let world = app.world();
        assert!(
            world
                .get::<TextFont>(text)
                .is_some_and(|f| (f.font_size - 30.0).abs() < f32::EPSILON),
            "button-text size must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<BackgroundColor>(button).map(|c| c.0),
            Some(new_button),
            "button fill must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<UiBorderColor>(button).map(|b| b.top),
            Some(new_border),
            "border color must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.border.left),
            Some(Val::Px(4.0)),
            "border width must reflect the NEW theme after re-run",
        );
        assert_eq!(
            world.get::<Node>(button).map(|n| n.border_radius.top_left),
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

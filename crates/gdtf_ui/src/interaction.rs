//! The theme-derived button interaction layer.
//!
//! [`theme_interaction`] is the per-widget hover/press feedback system. It
//! composes **on top of** the base look painted by
//! [`apply_theme`](crate::themed::apply_theme): for every button whose
//! [`Interaction`](bevy::ui::Interaction) changed this frame it writes the
//! state-appropriate fill from the *current*
//! [`GdtfTheme`](crate::theme::GdtfTheme) —
//! [`None`](bevy::ui::Interaction::None) → the resting
//! [`PanelBg`](crate::theme::PanelBg),
//! [`Hovered`](bevy::ui::Interaction::Hovered) →
//! [`HoverBg`](crate::theme::HoverBg),
//! [`Pressed`](bevy::ui::Interaction::Pressed) →
//! [`PressBg`](crate::theme::PressBg).
//!
//! ## Live theme read (never a spawn snapshot)
//!
//! The fill is resolved from the resource on **every** run, so a hot-reload
//! that changes the palette is reflected the next time the system writes — a
//! hovered or pressed button never shows a stale color from the old theme.
//!
//! ## Ordering and guards
//!
//! It runs after the [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)
//! set (bevy-traps rule 3) so it lays its swap on the freshest base, and takes
//! the theme as `Option<Res<GdtfTheme>>` so it is inert before the resource is
//! populated (bevy-traps rule 1). [`DisabledButton`](crate::widgets::DisabledButton)
//! widgets are excluded with `Without<DisabledButton>`, so a disabled button
//! never hover/press-swaps.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use crate::{theme::GdtfTheme, widgets::DisabledButton};

/// Query filter selecting the buttons [`theme_interaction`] restyles: enabled
/// buttons whose [`Interaction`](bevy::ui::Interaction) changed this frame.
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusion explicit:
/// `Without<DisabledButton>` is what skips disabled buttons (AC#5), and
/// `Changed<Interaction>` is what limits the work to state transitions.
type InteractedButton = (Changed<Interaction>, With<Button>, Without<DisabledButton>);

/// The per-button visuals [`theme_interaction`] reads and writes: the current
/// [`Interaction`](bevy::ui::Interaction) plus the
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and
/// [`BorderColor`](bevy::ui::BorderColor) it layers the theme state-fill onto.
type InteractionVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

/// Lays theme-derived hover/press feedback on top of the base button look.
///
/// Queries every [`Button`](bevy::ui::Button) whose
/// [`Interaction`](bevy::ui::Interaction) `Changed` this frame and is **not** a
/// [`DisabledButton`](crate::widgets::DisabledButton), and writes its
/// [`BackgroundColor`](bevy::ui::BackgroundColor) (and re-affirms its
/// [`BorderColor`](bevy::ui::BorderColor)) from the **current**
/// [`GdtfTheme`](crate::theme::GdtfTheme):
///
/// - [`Interaction::None`](bevy::ui::Interaction::None) → the resting
///   [`PanelBg`](crate::theme::PanelBg) base.
/// - [`Interaction::Hovered`](bevy::ui::Interaction::Hovered) →
///   [`HoverBg`](crate::theme::HoverBg).
/// - [`Interaction::Pressed`](bevy::ui::Interaction::Pressed) →
///   [`PressBg`](crate::theme::PressBg).
///
/// All three fills are sourced from the RON-driven theme — there are no
/// hardcoded hover/press literals. The theme is read live each run (never
/// snapshotted at spawn), so re-running after a palette change re-derives the
/// fill; the border is re-affirmed from the theme's single
/// [`BorderColor`](crate::theme::BorderColor).
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)`
/// and guarded by `Option<Res<GdtfTheme>>` for the absent-resource case
/// (bevy-traps rules 1 and 3).
pub fn theme_interaction(
    theme: Option<Res<GdtfTheme>>,
    mut buttons: Query<InteractionVisuals, InteractedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for (interaction, mut background, mut border) in &mut buttons {
        let fill = match interaction {
            Interaction::None => *theme.panel_bg,
            Interaction::Hovered => *theme.hover_bg,
            Interaction::Pressed => *theme.press_bg,
        };
        background.0 = fill;
        *border = UiBorderColor::all(*theme.border_color);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui::{BackgroundColor, Interaction, widget::Button};

    use super::*;
    use crate::{
        theme::{GdtfTheme, GdtfThemeSpec},
        themed::{UiSystems, apply_theme},
        widgets::{ButtonLabel, DisabledButton, spawn_button},
    };

    /// Builds a [`GdtfTheme`] with caller-chosen panel / hover / press colors
    /// through the real resolution path (deserialize a spec, then
    /// [`GdtfThemeSpec::resolve`]) with a defaulted font handle. Returns the
    /// `ron` error so a malformed literal surfaces via `?` rather than a denied
    /// `unwrap`/`panic`.
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

    /// Builds a minimal app with the real production schedule: `apply_theme`
    /// (in its named set) before `theme_interaction`, both under the live run
    /// condition — mirroring [`UiPlugin`](crate::UiPlugin)'s wiring.
    fn app_with_interaction() -> App {
        let mut app = App::new();
        app.add_systems(
            Update,
            (
                apply_theme.in_set(UiSystems::ApplyTheme),
                theme_interaction.after(UiSystems::ApplyTheme),
            )
                .run_if(resource_exists::<GdtfTheme>),
        );
        app
    }

    /// Sets a button's [`Interaction`] in the world (the swap a real pointer
    /// would otherwise drive), so the test can exercise the state transitions
    /// headlessly.
    fn set_interaction(app: &mut App, button: Entity, state: Interaction) {
        if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
            *interaction = state;
        }
    }

    /// `theme_interaction` swaps a hovered button to `HoverBg` and a pressed
    /// button to `PressBg`, both sourced from the theme — AC#4.
    ///
    /// Pin-discriminating: a wrong state→color mapping, or hardcoded literals,
    /// fail an assert.
    #[test]
    fn hover_and_press_swap_to_theme_state_colors() -> Result<(), ron::error::SpannedError> {
        let panel = [0.08, 0.08, 0.10, 1.0];
        let hover = [0.20, 0.20, 0.24, 1.0];
        let press = [0.04, 0.04, 0.06, 1.0];
        let theme_res = theme(panel, hover, press)?;

        let mut app = app_with_interaction();
        app.insert_resource(theme_res.clone());

        let button = {
            let mut commands = app.world_mut().commands();
            spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
        };
        app.world_mut().flush();

        set_interaction(&mut app, button, Interaction::Hovered);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*theme_res.hover_bg),
            "hovered button must show HoverBg",
        );

        set_interaction(&mut app, button, Interaction::Pressed);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*theme_res.press_bg),
            "pressed button must show PressBg",
        );

        set_interaction(&mut app, button, Interaction::None);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*theme_res.panel_bg),
            "released button must return to the resting PanelBg",
        );

        Ok(())
    }

    /// A `DisabledButton` is skipped by `theme_interaction`: simulating Hovered
    /// leaves its background at the `apply_theme` base, never `HoverBg` — AC#5.
    ///
    /// Pin-discriminating: dropping `Without<DisabledButton>` would let the
    /// hover swap fire and this assert would see `HoverBg`.
    #[test]
    fn disabled_button_is_skipped_by_interaction() -> Result<(), ron::error::SpannedError> {
        let panel = [0.08, 0.08, 0.10, 1.0];
        let hover = [0.20, 0.20, 0.24, 1.0];
        let press = [0.04, 0.04, 0.06, 1.0];
        let theme_res = theme(panel, hover, press)?;

        let mut app = app_with_interaction();
        app.insert_resource(theme_res.clone());

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
        // Establish the base look first.
        app.update();
        let base = app.world().get::<BackgroundColor>(button).map(|c| c.0);

        set_interaction(&mut app, button, Interaction::Hovered);
        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            base,
            "disabled button's background must be unchanged by a Hovered interaction",
        );
        assert_ne!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*theme_res.hover_bg),
            "disabled button must never show HoverBg",
        );

        Ok(())
    }

    /// Hot-reload contract: after spawn + `apply_theme`, replacing `GdtfTheme`
    /// with a NEW palette repaints the resting button to the NEW base, and a
    /// subsequently-hovered button shows the NEW `HoverBg` — proving the
    /// interaction layer reads the live theme each run, never a spawn snapshot
    /// (AC#4b, test strategy #4).
    ///
    /// Pin-discriminating: if `theme_interaction` cached colors at spawn, the
    /// post-reload hover would still show the OLD `HoverBg` and the assert fails.
    #[test]
    fn hot_reload_repaints_resting_and_hovered_from_new_theme()
    -> Result<(), ron::error::SpannedError> {
        let old = theme(
            [0.08, 0.08, 0.10, 1.0],
            [0.20, 0.20, 0.24, 1.0],
            [0.04, 0.04, 0.06, 1.0],
        )?;
        let mut app = app_with_interaction();
        app.insert_resource(old.clone());

        let button = {
            let mut commands = app.world_mut().commands();
            spawn_button(&mut commands, &old, ButtonLabel::new("Fight"), ())
        };
        app.world_mut().flush();
        app.update();

        // Hot-reload: a deliberately different palette.
        let new = theme(
            [0.50, 0.10, 0.30, 1.0],
            [0.90, 0.70, 0.10, 1.0],
            [0.10, 0.40, 0.80, 1.0],
        )?;
        app.insert_resource(new.clone());

        // Re-run: apply_theme repaints the resting base from the NEW theme.
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*new.panel_bg),
            "resting button must reflect the NEW base after hot-reload",
        );

        // A subsequently-hovered button must show the NEW HoverBg.
        set_interaction(&mut app, button, Interaction::Hovered);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*new.hover_bg),
            "hovered button must reflect the NEW HoverBg after hot-reload",
        );
        assert_ne!(
            app.world().get::<BackgroundColor>(button).map(|c| c.0),
            Some(*old.hover_bg),
            "hovered button must NOT show the stale OLD HoverBg",
        );

        Ok(())
    }

    /// With no `GdtfTheme`, an `app.update()` does not panic: the
    /// `Option<Res<GdtfTheme>>` guard keeps `theme_interaction` inert
    /// (bevy-traps rule 1).
    #[test]
    fn absent_theme_does_not_panic() {
        let mut app = App::new();
        app.add_systems(Update, theme_interaction);
        app.world_mut()
            .spawn((Button, Interaction::Hovered, BackgroundColor(Color::WHITE)));

        app.update();

        assert!(
            app.world().get_resource::<GdtfTheme>().is_none(),
            "test precondition: GdtfTheme must be absent for this guard check",
        );
    }
}

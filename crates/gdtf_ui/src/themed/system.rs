//! The central [`apply_theme`] base-look pass, its `box_node` helper, and the
//! [`any_themed_added`] run condition.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use super::role::{ThemeRole, Themed};
use crate::theme::{ContentMargin, GdtfTheme};

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
/// [`UiSystems::ApplyTheme`](super::UiSystems::ApplyTheme), **change-driven**
/// (GTW-144): it runs only when the theme changed or a new [`Themed`] entity
/// appeared, never on steady-state frames (so it never clobbers interaction
/// feedback), and the `resource_exists::<GdtfTheme>` guard keeps it inert and
/// panic-free before the theme is populated (pre-`Load`, bevy-traps rule 1). See
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

//! The central [`apply_theme`] base-look pass, its `box_node` helper, and the
//! [`any_themed_added`] run condition.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use super::role::{ThemeRole, Themed};
use crate::theme::{ContentMargin, GdtfTheme};

/// Read-only [`Query`] data for one [`Themed`] entity: its [`Entity`], its
/// [`ThemeRole`] marker, and its optional layout [`Node`].
///
/// Named so [`apply_theme`]'s two repaint queries (the full-set theme-change pass
/// and the incremental `Added`/`Changed` pass) share one [`QueryData`] shape and
/// one paint helper ([`paint_themed`]) without restating the tuple at each call
/// site (clippy `type_complexity`).
type ThemedData<'a> = (Entity, &'a Themed, Option<&'a Node>);

/// Query FILTER matching a [`Themed`] entity that was freshly added OR whose
/// [`Themed`]/[`Node`] changed this frame
/// ([`Or<(Added<Themed>, Changed<Themed>)>`](Or)).
///
/// Named so [`apply_theme`]'s incremental repaint query stays legible (clippy
/// `type_complexity`): the GTW-284 incremental pass paints exactly these entities
/// on a steady-theme frame, so an unrelated spawn never recolors an existing
/// widget back to the resting base.
type AddedOrChangedThemed = Or<(Added<Themed>, Changed<Themed>)>;

/// Paints the base, theme-derived look onto the [`Themed`] entities that need it
/// this frame, from the **current** [`GdtfTheme`] resource, by [`ThemeRole`] (see
/// the module docs for the role→sub-theme mapping).
///
/// **Incremental by default (GTW-284).** On a frame where the theme itself did not
/// change, it repaints ONLY the entities that are freshly added or whose
/// [`Themed`]/[`Node`] changed
/// ([`Or<(Added<Themed>, Changed<Themed>)>`](Or)) — so spawning one new widget no
/// longer drags every existing button's [`BackgroundColor`](bevy::ui::BackgroundColor)
/// back to the resting base (which clobbered the GTW-118 hover and GTW-253 active
/// fills for a frame). When the theme resource DID change ([`Res::is_changed`] — the
/// `Load` insert and the GTW-137 hot-reload re-derive), it repaints the FULL
/// [`Themed`] set, so a palette swap re-themes every widget (the retheme). A
/// freshly-spawned widget always gets its initial paint: on the `Load` frame the
/// theme is changed, so the full-set arm covers it; a later spawn is covered by the
/// `Added` arm.
///
/// Components are written through [`Commands`], so the look is applied whether or
/// not the entity already carried the target component — apply-or-insert, robust
/// to spawn order. The theme is read **live** (a fresh `Res` borrow each run), so
/// re-running this after the resource changes re-themes; it never snapshots at
/// spawn. The two queries read the SAME [`Themed`]/[`Node`] components and write
/// only through [`Commands`], so they do not conflict (one filtered incremental,
/// one full; only one runs per frame).
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
    full: Query<ThemedData>,
    incremental: Query<ThemedData, AddedOrChangedThemed>,
) {
    let mut painted = 0usize;
    if theme.is_changed() {
        // A real theme change (the `Load` insert or the GTW-137 hot-reload re-derive):
        // repaint EVERY `Themed` widget so the new palette reaches all of them (the
        // retheme), and so a widget spawned on the same `Load` frame gets its initial paint.
        for data in &full {
            paint_themed(&mut commands, &theme, data);
            painted += 1;
        }
    } else {
        // Steady theme, but at least one `Themed` entity was added or changed this frame
        // (the run condition's `any_themed_added` arm fired): paint ONLY those, so an
        // unrelated spawn never recolors an existing widget back to the resting base and
        // clobbers its hover / active fill (the GTW-284 fix).
        for data in &incremental {
            paint_themed(&mut commands, &theme, data);
            painted += 1;
        }
    }
    // GTW-146 hot-reload instrumentation: this system is change-driven, so a line
    // here after a save confirms the re-derived theme was actually reapplied to
    // the live widgets (the final step of the reload chain).
    if painted > 0 {
        info!("apply_theme: repainted {painted} Themed entities from the current GdtfTheme");
    }
}

/// Paints one [`Themed`] entity's base look from `theme` by its [`ThemeRole`],
/// through [`Commands`] (apply-or-insert).
///
/// The single per-entity paint body, shared by [`apply_theme`]'s two repaint
/// passes (the full-set theme-change pass and the incremental `Added`/`Changed`
/// pass) so the role→sub-theme mapping lives in one place.
fn paint_themed(commands: &mut Commands, theme: &GdtfTheme, (entity, marker, node): ThemedData) {
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
                *theme.panel.border_width,
                *theme.panel.corner_radius,
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
                *theme.button.border_width,
                *theme.button.corner_radius,
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

/// Builds a themed box [`Node`](bevy::ui::Node): the existing node's layout
/// preserved, with only the theme-owned border width, corner radius, and content
/// padding overridden.
///
/// Shared by the [`ThemeRole::Panel`] and [`ThemeRole::Button`] arms — both paint
/// a box, differing only in which sub-theme's scalars feed in. In Bevy 0.18 the
/// corner radius lives in [`Node::border_radius`](bevy::ui::Node), the border
/// width in [`Node::border`](bevy::ui::Node), and the padding in
/// [`Node::padding`](bevy::ui::Node) — not standalone components.
///
/// All sizes are RELATIVE units (GTW-296): the border width + corner radius are
/// `Vw` (one axis, so a border/radius pair keeps its ratio), and the per-edge
/// padding is `Vw` on the horizontal edges + `Vh` on the vertical, so each axis
/// tracks the matching window dimension on resize.
fn box_node(node: Option<&Node>, border_vw: f32, radius_vw: f32, margin: ContentMargin) -> Node {
    let mut themed_node = node.cloned().unwrap_or_default();
    themed_node.border = UiRect::all(Val::Vw(border_vw));
    themed_node.border_radius = BorderRadius::all(Val::Vw(radius_vw));
    themed_node.padding = UiRect {
        left:   Val::Vw(*margin.l),
        right:  Val::Vw(*margin.r),
        top:    Val::Vh(*margin.t),
        bottom: Val::Vh(*margin.b),
    };
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

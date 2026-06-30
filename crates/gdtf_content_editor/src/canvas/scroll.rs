//! The [`spawn_canvas_scroll`] `OnEnter(Editing)` system — wraps the [`CanvasRegion`] in a
//! `gdtf_ui` scroll list so the overflowing cell grid scrolls rather than squeezing.
//!
//! Separated from the canvas grid builder and the interactivity systems so the scroll-list
//! wiring has its own focused home.

use bevy::prelude::*;
use gdtf_ui::{ScrollListColors, spawn_scroll_list, theme::GdtfTheme};

use super::types::CanvasScroll;
use crate::{CanvasRegion, mode::PrefabModeContent, mode_host::mode_host_under_region};

/// `OnEnter(Editing)`: wrap the [`CanvasRegion`] panel in a `gdtf_ui`
/// [`spawn_scroll_list`](gdtf_ui::spawn_scroll_list) so the (overflowing) cell grid SCROLLS
/// rather than squeezing (the chosen scale model).
///
/// The shell's [`CanvasRegion`] is a themed panel; this hangs a scroll list inside it (deferred,
/// so the panel exists) and re-parents the scroll-list root under the panel — the
/// `parent_scroll_root_under` precedent. [`sync_canvas`](super::sync::sync_canvas) then parents
/// the grid under the returned [`ScrollListArea`](gdtf_ui::ScrollListArea).
pub(crate) fn spawn_canvas_scroll(mut commands: Commands, theme: Res<GdtfTheme>) {
    let colors = ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.panel.border_color,
        thumb: *theme.panel.border_color,
    };
    let area = spawn_scroll_list(&mut commands, colors, CanvasScroll);
    commands.queue(move |world: &mut World| {
        // The canvas is PREFAB-mode content; hang its scroll-list root on the canvas region's
        // PREFAB-mode container so the whole paint canvas hides in TERRAIN mode (GTW-474).
        let Some(host) = mode_host_under_region::<CanvasRegion, PrefabModeContent>(world) else {
            return;
        };
        // The scroll-list area's root frame is parented within the same buffer by
        // `spawn_scroll_list`; move that root under the canvas region's prefab container.
        let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(root);
        }
    });
}

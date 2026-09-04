//! Top-level UI plugin: widgets, theming, focus nav.

use bevy::prelude::*;
use cobalt_ron_assets::{RonAsset, redrive_hot_ron_resource};

use crate::{
    focus_nav::FocusNavPlugin,
    theming::{
        retheme::theme_hot_ron_chain,
        theme::{GdtfTheme, GdtfThemeSpec},
        themed::{UiSystems, any_themed_added, apply_theme},
    },
    widgets::{
        core::{
            SegmentSelected, ToggleFlipped, drive_switches, paint_active_buttons,
            paint_disabled_buttons, repaint_segments, select_segment_on_press,
        },
        interaction::{
            paint_focus_ring, repaint_deactivated_buttons, repaint_enabled_buttons,
            repaint_theme_change, sync_hover_to_focus, theme_interaction,
        },
    },
};

type ThemeAssetMessages = Messages<AssetEvent<RonAsset<GdtfThemeSpec>>>;

/// Registers UI systems, messages, and focus navigation.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToggleFlipped>()
            .add_message::<SegmentSelected>()
            .add_systems(
                Update,
                (
                    drive_switches,
                    select_segment_on_press,
                    repaint_segments.after(select_segment_on_press),
                ),
            );
        app.insert_resource(theme_hot_ron_chain());
        app.add_plugins(FocusNavPlugin).add_systems(
            Update,
            (
                redrive_hot_ron_resource::<GdtfThemeSpec, GdtfTheme>
                    .before(UiSystems::ApplyTheme)
                    .run_if(
                        resource_exists::<GdtfTheme>
                            .and_then(resource_exists::<ThemeAssetMessages>),
                    ),
                apply_theme.in_set(UiSystems::ApplyTheme).run_if(
                    resource_exists::<GdtfTheme>
                        .and_then(resource_changed::<GdtfTheme>.or_else(any_themed_added)),
                ),
                (
                    theme_interaction,
                    paint_disabled_buttons,
                    paint_active_buttons.after(theme_interaction),
                    repaint_deactivated_buttons,
                    repaint_enabled_buttons,
                    sync_hover_to_focus,
                    paint_focus_ring.after(sync_hover_to_focus),
                )
                    .after(UiSystems::ApplyTheme),
                repaint_theme_change
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<GdtfTheme>.and_then(resource_changed::<GdtfTheme>)),
            ),
        );
    }
}

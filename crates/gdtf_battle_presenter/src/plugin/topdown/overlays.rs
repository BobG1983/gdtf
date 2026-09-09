use bevy::prelude::*;
use gdtf_battle_sim::{
    effects::fields::FieldRegistry, prelude::BattleInProgress, visibility::SquadVisibility,
};

// The overlay's draw system is debug-only, so only a debug build names it.
#[cfg(debug_assertions)]
use crate::draw_reachable_overlay;
use crate::{
    CrossLevelSignals, HighlightRequest, PresenterSystems, ReachableCells, ReachableOverlayEnabled,
    derive_cross_level_signals, draw_cross_level_signals, draw_field_overlay, draw_fire_target,
    draw_highlight_on_request, draw_path_preview,
};

pub(super) fn register_highlight_systems(app: &mut App) {
    app.add_message::<HighlightRequest>().add_systems(
        Update,
        draw_highlight_on_request
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

pub(super) fn register_path_preview_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_path_preview.in_set(PresenterSystems::Overlay).run_if(
            resource_exists::<BattleInProgress>.and_then(resource_exists::<SquadVisibility>),
        ),
    );
}

pub(super) fn register_fire_target_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_fire_target
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Field overlay is shipping (not debug-gated).
pub(super) fn register_field_overlay_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_field_overlay
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<FieldRegistry>),
    );
}

pub(super) fn register_cross_level_signals_systems(app: &mut App) {
    app.init_resource::<CrossLevelSignals>()
        .add_systems(
            Update,
            derive_cross_level_signals
                .in_set(PresenterSystems::Compose)
                .run_if(resource_exists::<BattleInProgress>),
        )
        .add_systems(
            Update,
            draw_cross_level_signals
                .in_set(PresenterSystems::Overlay)
                .run_if(resource_exists::<BattleInProgress>),
        );
}

/// Register the reachable-range debug overlay, off until a QA client turns it on.
pub(super) fn register_reachable_overlay_systems(app: &mut App) {
    app.insert_resource(ReachableOverlayEnabled::new(false))
        .init_resource::<ReachableCells>();
    // The flag is written in every build; only a debug build draws from it.
    #[cfg(debug_assertions)]
    app.add_systems(
        Update,
        draw_reachable_overlay
            .in_set(PresenterSystems::Overlay)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(reachable_overlay_enabled),
            ),
    );
}

/// Whether the reachable-range debug overlay is enabled.
#[cfg(debug_assertions)]
fn reachable_overlay_enabled(flag: Option<Res<ReachableOverlayEnabled>>) -> bool {
    flag.is_some_and(|flag| **flag)
}

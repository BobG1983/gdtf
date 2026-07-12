//! Shared `cross_level_signals` integration fixture: the headless
//! `TopDownRendererPlugin` app (the `reachable_overlay.rs` pattern). The derive
//! system reads only bare `Position`/`Faction`/`LifeState` components plus a
//! handful of sim resources, so this fixture authors those DIRECTLY via
//! `app.world_mut()` (the accepted headless idiom, `bevy-traps.md` #7 carve-out
//! (a)) rather than driving a full `setup_battle` pour.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::{Text2d, Visibility, With, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{CrossLevelBadgeLabel, CrossLevelBadgeTile, TopDownRendererPlugin};
use gdtf_battle_sim::prelude::BattleInProgress;

/// Bounded settle headroom for the derive + draw systems' deferred command flush.
pub(crate) const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real
/// `TopDownRendererPlugin` plus a `BattleInProgress` witness so the battle-gated
/// derive + draw systems run.
pub(crate) fn signals_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    // Warn-not-panic on a transient missing-resource gate race (the
    // `reachable_overlay.rs` precedent): this focused harness opens
    // `BattleInProgress` WITHOUT the full `setup_battle`, so other battle-gated
    // draw systems warn-skip rather than panicking.
    app.set_error_handler(warn);
    app
}

/// Drive `MAX_UPDATES` bounded updates — settles the derive system's write AND
/// the draw system's pooled-entity command flush.
pub(crate) fn settle(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        app.update();
    }
}

/// Count of currently-`Visible` pooled [`CrossLevelBadgeTile`] entities, across
/// the WHOLE app — shared by every `cross_level_signals` integration test that
/// asserts the DRAWN pool, not just the [`CrossLevelSignals`](gdtf_battle_presenter::CrossLevelSignals)
/// resource (`cap.rs` / `connector.rs`; module-layout rule 6, a 2+-consumer
/// helper lives in the shared harness).
pub(crate) fn visible_tile_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<&Visibility, With<CrossLevelBadgeTile>>()
        .iter(app.world())
        .filter(|v| **v == Visibility::Visible)
        .count()
}

/// Every currently-`Visible` badge label's text (the `fire_target.rs`
/// `(**text).clone()` idiom) — unordered (query iteration order is
/// unspecified); callers assert set membership, not position. Shared by every
/// `cross_level_signals` integration test that asserts the DRAWN pool
/// (`cap.rs` / `connector.rs`; module-layout rule 6).
pub(crate) fn visible_label_texts(app: &mut App) -> Vec<String> {
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &Visibility, &CrossLevelBadgeLabel)>();
    q.iter(app.world())
        .filter(|(_, vis, _)| **vis == Visibility::Visible)
        .map(|(text, ..)| (**text).clone())
        .collect()
}

//! Theme load: good path resolves fonts; bad path falls back to default and still releases.
use std::path::PathBuf;

use bevy::{asset::Handle, text::Font};
use cobalt_test_utils::{
    LoadTestAppBuilder, MinimalTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_game::test_support::{AppState, app_state, load_released};
use gdtf_ui::theme::{GdtfTheme, default_theme};

use crate::load_families::load_suite::gate;

/// Frames the machine is given to prove it stays put — per-frame work, no IO.
const HOLD_FRAMES: u32 = 32;

#[test]
fn entering_load_without_asset_server_does_not_panic() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    app.update();

    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "no AssetServer means no load resolves, so no GdtfTheme should be inserted",
    );
}

#[test]
fn theme_present_transitions_to_intro_and_persists() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "precondition: rest in Load"
    );

    gate::seed_full_load_gate(&mut app);

    advance_until(&mut app, load_released);

    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme must persist past OnExit(Load) — it is the state-scoped-resource exception",
    );
}

#[test]
fn load_does_not_leave_without_a_theme() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<GdtfTheme>(&mut app);

    for _ in 0..HOLD_FRAMES {
        app.update();
    }

    assert_eq!(
        app_state(&app),
        AppState::Load,
        "without a GdtfTheme the machine must not leave Load",
    );
}

fn bad_theme_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_theme_root")
}

#[test]
fn real_asset_good_path_resolves_shipped_theme_and_transitions() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            &**theme.default_font, "fonts/Alegreya-Variable.ttf",
            "good path must carry the shipped default_font key",
        );
        for (label, font) in [
            ("button", theme.button.font.clone()),
            ("title", theme.title.font.clone()),
            ("text", theme.text.font.clone()),
        ] {
            assert_ne!(
                font,
                Handle::<Font>::default(),
                "good path must thread the real loaded {label} font handle into the theme",
            );
        }
    }

    advance_until(&mut app, load_released);
}

#[test]
fn real_asset_multi_font_load_resolves_distinct_title_font() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_ne!(
            theme.title.font, theme.button.font,
            "title's overriding font must resolve to a DISTINCT handle from the default font",
        );
        assert_eq!(
            theme.button.font, theme.text.font,
            "button + text both use the default_font, so they share one handle",
        );
        assert_ne!(
            theme.title.font,
            Handle::<Font>::default(),
            "the override font must be a real loaded handle, not the default",
        );
    }
}

#[test]
fn real_asset_failure_path_does_not_hang_and_uses_default_theme() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        bad_theme_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            theme,
            &default_theme(),
            "the failure path must insert exactly the const-fallback default theme",
        );
    }

    advance_until(&mut app, load_released);
}

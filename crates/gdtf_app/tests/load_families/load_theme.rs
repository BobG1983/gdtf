//! Theme load: good path resolves fonts; bad path falls back to default and still releases.
#[path = "load_suite/gate.rs"]
mod gate;

use std::path::PathBuf;

use bevy::{asset::Handle, text::Font};
use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::{GdtfTheme, default_theme};

const TRANSITION_BUDGET: u32 = 32;

const LOAD_SAFETY_NET: u32 = 10_000;

#[test]
fn entering_load_without_asset_server_does_not_panic() {
    let mut app = GdtfTestAppBuilder::new()
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
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "precondition: rest in Load"
    );

    gate::seed_full_load_gate(&mut app);

    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "with a GdtfTheme present, Load must release to Intro (or beyond) within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );

    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme must persist past OnExit(Load) — it is the state-scoped-resource exception",
    );
}

#[test]
fn load_does_not_leave_without_a_theme() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<GdtfTheme>(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "without a GdtfTheme the machine must not leave Load; it reached {:?}",
        app_state(&app),
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
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

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

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a resolved GdtfTheme, Load must release to Intro (or beyond); last AppState \
         was {:?}",
        app_state(&app),
    );
}

#[test]
fn real_asset_multi_font_load_resolves_distinct_title_font() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

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
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_theme_root())
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            theme,
            &default_theme(),
            "the failure path must insert exactly the const-fallback default theme",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "even on a failed asset, Load must release to Intro (or beyond) with the default \
         theme; last AppState was {:?}",
        app_state(&app),
    );
}

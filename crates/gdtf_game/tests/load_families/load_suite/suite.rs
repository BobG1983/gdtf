//!    add three one-line `#[test]`s invoking [`loader_no_ops_without_asset_server`],
use gdtf_assets::ContentFamily;
use gdtf_game::test_support::{AppState, app_state, load_released};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::GdtfTheme;

use super::gate;

/// Frames the machine is given to prove it stays put — per-frame work, no IO.
const HOLD_FRAMES: u32 = 32;

pub(crate) trait FamilyLoadContract: ContentFamily {
    fn is_empty(registry: &Self::Registry) -> bool;
}

pub(crate) fn loader_no_ops_without_asset_server<F: FamilyLoadContract>() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    assert!(
        app.world().get_resource::<F::Registry>().is_some(),
        "with no AssetServer, `register_content_family` must seed the default {}",
        registry_name::<F>(),
    );

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the `{}` kick-off must no-op and the machine rests in Load, not panic",
        F::FOLDER,
    );

    gate::seed_full_load_gate(&mut app);

    advance_until(&mut app, load_released);
}

pub(crate) fn load_gates_on_registry<F: FamilyLoadContract>() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<F::Registry>(&mut app);

    for _ in 0..HOLD_FRAMES {
        app.update();
    }
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no {} present, the machine stays in Load (the `{}` folder must be verified loaded \
         before Load exits)",
        registry_name::<F>(),
        F::FOLDER,
    );

    gate::seed_full_load_gate(&mut app);
    app.world_mut().insert_resource(F::Registry::default());
    advance_until(&mut app, load_released);
}

pub(crate) fn real_asset_resolves_registry<F: FamilyLoadContract>() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<F::Registry>(&mut app);

    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "the real `{}` folder load must insert a {} (last AppState was {:?})",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        assert!(
            !F::is_empty(registry),
            "the resolved {} must carry authored (non-empty) `{}` members",
            registry_name::<F>(),
            F::FOLDER,
        );
    }

    advance_until(&mut app, load_released);
    assert!(
        app.world().get_resource::<F::Registry>().is_some(),
        "a {} must be present after Load releases (the gate waited for it; Load's cleanup \
         deliberately persists the content registries)",
        registry_name::<F>(),
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme resolved alongside the `{}` folder (the gated transition fired)",
        F::FOLDER,
    );
}

fn registry_name<F: FamilyLoadContract>() -> &'static str {
    let full = std::any::type_name::<F::Registry>();
    full.rsplit("::").next().unwrap_or(full)
}

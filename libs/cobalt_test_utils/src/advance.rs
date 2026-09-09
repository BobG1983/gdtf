//! Advance a test app until a condition holds.
//!
//! Every wait here is budget-free: a loaded machine makes a test slower, never red.

use bevy::{
    app::App,
    asset::{AssetServer, LoadState, UntypedAssetId},
    ecs::resource::Resource,
};

/// Run `app.update()` until `predicate` is true.
pub fn advance_until(app: &mut App, predicate: impl Fn(&App) -> bool) {
    loop {
        app.update();
        if predicate(app) {
            return;
        }
    }
}

/// Run `app.update()` until `predicate` is true, for a predicate that needs `&mut App`.
///
/// The same uncapped shape as [`advance_until`], for a probe that goes through `World::query`.
pub fn advance_until_mut(app: &mut App, mut predicate: impl FnMut(&mut App) -> bool) {
    loop {
        app.update();
        if predicate(app) {
            return;
        }
    }
}

/// Run updates until resource `T` exists.
pub fn advance_until_resource_exists<T: Resource>(app: &mut App) {
    advance_until(app, |app| app.world().get_resource::<T>().is_some());
}

/// Run updates until the asset at `id` reaches a terminal load state.
///
/// # Panics
///
/// Panics if the terminal state is not accepted by `predicate` — the load
/// failed where the test awaited success, or the reverse.
pub fn advance_until_load_state(
    app: &mut App,
    id: impl Into<UntypedAssetId>,
    predicate: impl Fn(LoadState) -> bool,
) {
    let id = id.into();
    let load_state = |app: &App| app.world().resource::<AssetServer>().get_load_state(id);
    advance_until(app, |app| {
        load_state(app)
            .is_some_and(|state| matches!(state, LoadState::Loaded | LoadState::Failed(_)))
    });
    let terminal = load_state(app);
    assert!(
        terminal.clone().is_some_and(&predicate),
        "asset `{id:?}` finished loading in a terminal state the test did not await: {terminal:?}",
    );
}

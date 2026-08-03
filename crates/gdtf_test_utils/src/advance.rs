use bevy::{
    app::App,
    asset::{AssetServer, LoadState, UntypedAssetId},
    ecs::resource::Resource,
};

pub fn advance_until(app: &mut App, predicate: impl Fn(&App) -> bool, max_updates: u32) -> bool {
    for _ in 0..max_updates {
        app.update();
        if predicate(app) {
            return true;
        }
    }
    false
}

pub fn advance_until_resource_exists<T: Resource>(app: &mut App, max_updates: u32) {
    for _ in 0..max_updates {
        app.update();
        if app.world().get_resource::<T>().is_some() {
            return;
        }
    }
    assert!(
        app.world().get_resource::<T>().is_some(),
        "resource `{}` was never inserted within {max_updates} updates — the async asset load did \
         not resolve (neither the success-resolve nor the failure-default path fired). This is a \
         genuine load failure, not a frame-budget shortfall (the cap is a safety net, not a timing \
         budget).",
        core::any::type_name::<T>(),
    );
}

pub fn advance_until_load_state(
    app: &mut App,
    id: impl Into<UntypedAssetId>,
    predicate: impl Fn(LoadState) -> bool,
    max_updates: u32,
) {
    let id = id.into();
    for _ in 0..max_updates {
        app.update();
        if app
            .world()
            .resource::<AssetServer>()
            .get_load_state(id)
            .is_some_and(&predicate)
        {
            return;
        }
    }
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_load_state(id)
            .is_some_and(&predicate),
        "asset `{id:?}` never reached the awaited load state within {max_updates} updates — the \
         async asset load did not resolve to the expected terminal state. This is a genuine \
         unresolved load (e.g. an unregistered loader, a wrong source root, or a panicking \
         loader), not a frame-budget shortfall (the cap is a safety net, not a timing budget).",
    );
}

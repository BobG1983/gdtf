//! Multi-frame driving helpers for headless test apps.
//!
//! Two flavours, by what they wait ON:
//!
//! - [`advance_until`] — a generic predicate poll with a caller-chosen
//!   `max_updates`. Use it for state-machine / non-asset waits where the budget
//!   is a real, bounded number of frames (e.g. "did the machine leave `Load`?").
//! - [`advance_until_resource_exists`] — a **signal poll** for async asset loads:
//!   it drives the app until a terminal RESOURCE is inserted (the robust
//!   completion signal — the resolve systems insert the resource once the load
//!   finishes, on BOTH the success-resolve and the failure-default paths), and
//!   PANICS with a diagnostic on timeout. The `max_updates` here is a generous
//!   SAFETY NET, not a timing budget: an async load polled under parallel `cargo`
//!   contention has no fixed frame count, so waiting on the inserted-resource
//!   signal — not a frame count — is what makes the load tests deterministic.
//!   See GTW-305.
//! - [`advance_until_load_state`] — the same **signal poll** discipline as
//!   `advance_until_resource_exists`, but for code that observes the
//!   [`AssetServer`]'s LOAD STATE directly (rather than a resolve system's
//!   inserted resource): it drives the app until the asset's terminal
//!   [`LoadState`] (`Loaded` / `Failed`) satisfies a caller predicate, and asserts
//!   with a diagnostic on timeout.
//!   See GTW-305, GTW-319.

use bevy::{
    app::App,
    asset::{AssetServer, LoadState, UntypedAssetId},
    ecs::resource::Resource,
};

/// Drives `app` forward up to `max_updates` times, stopping early when
/// `predicate` holds.
///
/// Calls [`App::update`] and then evaluates `predicate(&app)` after each update,
/// returning `true` as soon as the predicate is satisfied. Returns `false` if the
/// predicate never holds within `max_updates` updates. Useful for waiting on a
/// multi-frame state transition without hard-coding a frame count.
pub fn advance_until(app: &mut App, predicate: impl Fn(&App) -> bool, max_updates: u32) -> bool {
    for _ in 0..max_updates {
        app.update();
        if predicate(app) {
            return true;
        }
    }
    false
}

/// Drives `app` forward until the resource `T` is present in its world, polling
/// the inserted-resource **signal** rather than a fixed frame count.
///
/// This is the deterministic wait for an async asset load: the resolve systems
/// insert `T` once the load completes — on the success path (the resolved value)
/// AND on the failure path (the const-default fallback) — so "the resource is
/// present" is the robust terminal condition for both outcomes. Because an async
/// load polled under parallel `cargo` contention takes a NON-deterministic number
/// of frames, waiting on this signal (not on `max_updates`) is what removes the
/// flake (GTW-305).
///
/// `max_updates` is a generous SAFETY NET, not a timing budget: pass something
/// high enough to absorb any parallel-load variance (e.g. `10_000`), yet finite
/// so a genuinely missing / never-resolving asset fails fast-ish with evidence
/// rather than hanging forever.
///
/// # Panics
///
/// Panics (via `assert!`, naming `T`) if the resource is still absent after
/// `max_updates` updates. In a test this is the CORRECT failure mode: a genuine
/// unmet load condition SHOULD fail loudly with a diagnostic, not silently return
/// a flag a caller might ignore. The message names the resource type so the failure
/// points straight at the load that never resolved.
pub fn advance_until_resource_exists<T: Resource>(app: &mut App, max_updates: u32) {
    for _ in 0..max_updates {
        app.update();
        if app.world().get_resource::<T>().is_some() {
            return;
        }
    }
    // Timed out: fail loudly with a diagnostic naming `T`. This is the correct test
    // failure mode for a genuine unmet load condition — and an `assert!` (not a bare
    // `panic!`) keeps it within the workspace's deny-`panic` lint while still aborting
    // the test with the message. The condition is re-read here so the assert reflects
    // the final, post-loop world.
    assert!(
        app.world().get_resource::<T>().is_some(),
        "resource `{}` was never inserted within {max_updates} updates — the async asset load did \
         not resolve (neither the success-resolve nor the failure-default path fired). This is a \
         genuine load failure, not a frame-budget shortfall (the cap is a safety net, not a timing \
         budget).",
        core::any::type_name::<T>(),
    );
}

/// Drives `app` forward until the [`AssetServer`]'s [`LoadState`] for `id`
/// satisfies `predicate`, polling that terminal-state **signal** rather than a
/// fixed frame count.
///
/// This is the deterministic wait for code that observes an async asset's load
/// state directly — as opposed to [`advance_until_resource_exists`], which waits
/// on a resolve system's inserted resource. Pass a predicate over the asset's
/// TERMINAL load state, e.g. [`LoadState::is_loaded`] for the success signal or
/// [`LoadState::is_failed`] for the failure signal. Because an async load polled
/// under parallel `cargo` contention takes a NON-deterministic number of frames,
/// waiting on this signal (not on `max_updates`) is what removes the flake
/// (GTW-305, GTW-319).
///
/// `max_updates` is a generous SAFETY NET, not a timing budget: pass something
/// high enough to absorb any parallel-load variance (e.g. `10_000`), yet finite
/// so a genuinely missing / never-resolving asset fails fast-ish with evidence
/// rather than hanging forever.
///
/// # Panics
///
/// Panics (via `assert!`, naming the asset `id`) if the load state never
/// satisfies `predicate` within `max_updates` updates. In a test this is the
/// CORRECT failure mode: a genuine unmet load condition SHOULD fail loudly with a
/// diagnostic — naming the unresolved `id` and stating this is a real load
/// failure, not a frame-budget shortfall — not silently return a flag a caller
/// might ignore. An `assert!` (not a bare `panic!`) keeps it within the
/// workspace's deny-`panic` lint while still aborting the test.
pub fn advance_until_load_state(
    app: &mut App,
    id: impl Into<UntypedAssetId>,
    predicate: impl Fn(LoadState) -> bool,
    max_updates: u32,
) {
    // Capture the id once: `get_load_state` consumes an `impl Into<UntypedAssetId>`,
    // and `UntypedAssetId` is `Copy`, so a single up-front conversion lets us re-poll
    // it each frame (and re-read it for the final diagnostic) without moving it.
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
    // Timed out: fail loudly with a diagnostic naming the asset `id`. The condition
    // is re-read here so the assert reflects the final, post-loop world.
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

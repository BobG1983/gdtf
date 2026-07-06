//! Shared Load-gate seed helpers (GTW-580) — thin drivers over the ONE
//! test-side seed source, [`seed_load_gate`].
//!
//! No resource is named here: the BESPOKE gate set lives in
//! `gdtf_app::test_support::seed_load_gate` (the production
//! `seed_load_fallbacks` plus its documented delta), so adding a gate-blocking
//! bespoke resource touches the seed source only — never a test file. The
//! seam-family registries are NOT seeded here at all: each is seeded at APP
//! BUILD by its own `register_content_family` line (the GTW-629
//! headless-fallback rider), so a new content family touches nothing. A
//! tier-(a) negative test withholds ONE resource by seeding everything and
//! then removing its own registry ([`seed_gate_except`]), which stays
//! deterministic because no `app.update()` runs between the seed and the
//! removal (the gated transition can only fire during `Update`).
//!
//! This file is self-contained (no `super::` references) so bespoke,
//! non-family load tests can include it standalone via
//! `#[path = "load_suite/gate.rs"] mod gate;`.

use bevy::{
    app::App,
    ecs::{resource::Resource, system::RunSystemOnce},
};
use gdtf_app::test_support::seed_load_gate;

/// Seeds EVERY Load-gate-blocking resource by running the real
/// [`seed_load_gate`] system once against the app world — the tier-(a)
/// stand-in for all asset resolves completing under `MinimalPlugins`.
pub(crate) fn seed_full_load_gate(app: &mut App) {
    let seeded = app.world_mut().run_system_once(seed_load_gate);
    assert!(
        seeded.is_ok(),
        "seed_load_gate's params (Commands + Option<Res<AssetServer>>) are infallible on any \
         world; got {seeded:?}",
    );
}

/// Seeds the full Load gate ([`seed_full_load_gate`]) and then WITHHOLDS `R` —
/// the tier-(a) negative-test shape proving `R` is a genuine gate-blocking
/// resource (every OTHER gate resource present, only `R` absent).
pub(crate) fn seed_gate_except<R: Resource>(app: &mut App) {
    seed_full_load_gate(app);
    app.world_mut().remove_resource::<R>();
}

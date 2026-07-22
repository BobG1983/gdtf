//! The GTW-580 generic per-family load suite — ONE parameterized encoding of
//! the tier structure the per-family `load_*.rs` clones used to hand-copy.
//!
//! # The two tiers (the structure every folder family shares)
//!
//! - **Tier (a)** — `MinimalPlugins` via `GdtfTestAppBuilder`: no
//!   `AssetServer`, so the family's loader registration + folder kick-off must
//!   no-op without panicking (`bevy-traps.md` rule 1) while the registration
//!   line seeds the family's DEFAULT registry (the GTW-629 headless-fallback
//!   rider); the gated Load→Intro transition is then driven by seeding the
//!   remaining (bespoke) gate set through the ONE seed source
//!   ([`gate`](super::gate)), and the companion negative withholds the
//!   family's registry to prove it is genuinely gate-blocking.
//! - **Tier (b)** — headless `DefaultPlugins` via `GdtfLoadTestAppBuilder`: a
//!   real `AssetServer` drives the WHOLE production Load flow — the GTW-570
//!   `register_content_family` registration loads the family's real folder from disk
//!   into its registry resource through the `AppState::Load` state machine.
//!   The suite NEVER seeds the registry (seeding would mask the very
//!   regression under test) and never re-implements the loader: existence of
//!   the resource proves the real generic resolve published it.
//!
//! VALUE-AGNOSTIC throughout: the suite pins registry PRESENCE and the
//! authored member KEYS only — never a shipped magnitude (the brittle-test
//! rule), so a content/tuning edit never reddens it.
//!
//! # Adding folder family N+1 (the P1 recipe)
//!
//! One thin wrapper file plus its fixtures — ZERO edits to any pre-existing
//! test file or fixture root:
//!
//! 1. `tests/load_<family>.rs`: `mod load_suite;`, implement
//!    [`FamilyLoadContract`] for the family's `gdtf_content_families` marker
//!    (expected authored member labels + the two registry projections), and
//!    add three one-line `#[test]`s invoking [`loader_no_ops_without_asset_server`],
//!    [`load_gates_on_registry`], and [`real_asset_resolves_registry`].
//! 2. Author the family's content folder under `assets/` (tier (b) reads the
//!    real folder through the real loader — no fixture copy of shipped content).
//! 3. NOTHING else — the family's ONE `register_content_family` line already
//!    yields its loader, per-file salvage, validation-window membership, and
//!    the headless fallback (the GTW-629 rider seeds `Registry::default()`
//!    when there is no `AssetServer`), so the Load gate stays satisfiable in
//!    every tier-(a) walk with zero seed-arm edits. (Only a new BESPOKE
//!    gate-blocking resource still extends the production
//!    `seed_load_fallbacks`.)

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_assets::ContentFamily;
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::GdtfTheme;

use super::gate;

/// Bounded budget for the tier-(a) `MinimalPlugins` transition / negative
/// waits, where all gate resources are injected by hand — a true, small,
/// deterministic frame count (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the tier-(b) real-asset `advance_until` waits
/// gated on an async asset load resolving. The family folder load shares the
/// `AssetServer` with the whole scene stack, so under parallel `cargo`
/// contention the async resolve has NO fixed frame count. These waits key off
/// the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The per-family parameters of the generic load suite: the family's
/// [`ContentFamily`] marker (folder / extension / registry ride the trait)
/// plus the value-agnostic projections the suite asserts through.
///
/// Implemented by each wrapper file for its `gdtf_content_families` marker —
/// the impl lives IN the wrapper so family N+1 never edits a shared file.
pub(crate) trait FamilyLoadContract: ContentFamily {
    /// Labels of authored members the SHIPPED folder is expected to carry —
    /// content KEYS (filename stems or named UUIDs), never magnitudes. The
    /// suite asserts each resolves; it never pins a count, so authoring MORE
    /// content cannot redden it.
    const EXPECTED_MEMBERS: &'static [&'static str];

    /// True when the resolved registry holds no members.
    fn is_empty(registry: &Self::Registry) -> bool;

    /// Whether the authored member behind `label` resolves in `registry`.
    /// The label→key mapping (stem newtype, UUID const, …) is family-owned.
    fn member_resolves(registry: &Self::Registry, label: &str) -> bool;
}

/// Tier (a) — under `MinimalPlugins` there is no `AssetServer`, so entering
/// `Load` must not panic: the family's loader registration and `load_folder`
/// kick-off both guard on the missing server and no-op (`bevy-traps.md` rule
/// 1), while the registration line seeds the family's DEFAULT registry (the
/// GTW-629 headless-fallback rider — the recipe's "one line yields the
/// fallback" pin). The machine still advances past `Load` once the remaining
/// gate set is seeded (standing in for all resolves completing), proving the
/// guard holds.
pub(crate) fn loader_no_ops_without_asset_server<F: FamilyLoadContract>() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // The GTW-629 rider: with no AssetServer the family's ONE registration
    // line (in the Load plugin, built above) seeded the default registry.
    assert!(
        app.world().get_resource::<F::Registry>().is_some(),
        "with no AssetServer, `register_content_family` must seed the default {} (the \
         GTW-629 headless-fallback rider)",
        registry_name::<F>(),
    );

    // Enter Load: the loader registration + folder kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the `{}` kick-off must no-op and the machine rests in Load, not panic",
        F::FOLDER,
    );

    // Stand in for every resolve completing: the ONE gate-seed source.
    gate::seed_full_load_gate(&mut app);

    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "with the full gate set seeded, Load must release to Intro (or beyond) within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// Tier (a) companion — the Load→Intro transition GATES on the family's
/// registry: with every OTHER gate resource present but the registry withheld
/// (removed after the build-time rider seed, with no `AssetServer` to resolve
/// one), the machine must stay in `Load` for the whole budget; re-seeding the
/// gate plus the registry then releases it. Proves the
/// registry is a genuine gate-blocking resource (the machine never leaves
/// `Load` before the family folder is verified loaded), bracketing the
/// transition condition from both sides.
pub(crate) fn load_gates_on_registry<F: FamilyLoadContract>() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Every other gate resource present, the family registry deliberately withheld.
    gate::seed_gate_except::<F::Registry>(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );
    assert!(
        !left_load,
        "Load must NOT leave while the {} is absent; it left to {:?}",
        registry_name::<F>(),
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no {} present, the machine stays in Load (the `{}` folder must be verified loaded \
         before Load exits)",
        registry_name::<F>(),
        F::FOLDER,
    );

    // The flip: with the registry seeded too, the SAME machine advances —
    // proving the registry was the one withheld gate condition. The seed
    // source only re-covers the BESPOKE gate resources (GTW-629 moved the
    // content-family arms out of `seed_load_fallbacks`), so the family's own
    // registry is re-seeded directly — the SAME default the family rider seeds
    // at registration (a build-time seed cannot re-run here).
    gate::seed_full_load_gate(&mut app);
    app.world_mut().insert_resource(F::Registry::default());
    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "once the {} is seeded, Load must release to Intro (or beyond) within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        registry_name::<F>(),
        app_state(&app),
    );
}

/// Tier (b) — with a real `AssetServer` rooted at the workspace `assets/`,
/// entering `Load` loads the family's real folder through the GTW-570
/// `register_content_family` registration end-to-end and builds its registry; each
/// expected authored member key resolves, and the Load gate WAITED for the
/// registry (the machine releases past `Load` with it present).
///
/// DELIBERATELY seeds nothing — existence of the registry proves the REAL
/// generic resolve published it from disk (a pre-seeded default would mask
/// exactly that regression, the seed-shadow class).
pub(crate) fn real_asset_resolves_registry<F: FamilyLoadContract>() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async folder load: wait until the registry is inserted,
    // not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "the real `{}` folder load must insert a {} within the safety-net budget \
         (last AppState was {:?})",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        assert!(
            !F::is_empty(registry),
            "the resolved {} must carry the authored (non-empty) `{}` members",
            registry_name::<F>(),
            F::FOLDER,
        );
        for member in F::EXPECTED_MEMBERS {
            assert!(
                F::member_resolves(registry, member),
                "the {} must resolve the authored `{member}` member (loaded from the real `{}` \
                 folder through the generic content-family loader)",
                registry_name::<F>(),
                F::FOLDER,
            );
        }
    }

    // The Load gate WAITED for the registry. Intro is a TRANSIENT stop, so
    // probe via `load_released` — accept Intro OR the state past it
    // (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once every folder \
         (incl. `{}`) resolves; last AppState was {:?}",
        F::FOLDER,
        app_state(&app),
    );
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

/// The family registry's short type name, for assertion messages.
fn registry_name<F: FamilyLoadContract>() -> &'static str {
    let full = std::any::type_name::<F::Registry>();
    full.rsplit("::").next().unwrap_or(full)
}

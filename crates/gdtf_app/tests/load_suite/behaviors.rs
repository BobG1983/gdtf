//! The GTW-619 DEEP-BEHAVIOR extension of the GTW-580 per-family load suite —
//! ONE parameterized encoding of the four load behaviors a family migrated
//! onto the GTW-570 generic content-family chain must preserve: per-file
//! SALVAGE parity (GTW-582 C4), fail-closed-EMPTY on an unenumerable folder,
//! NEVER-publish-partial, and the live REDRIVE rebuild. Every walk drives the
//! REAL production Load wiring (the `register_content_family` call in the Load
//! plugin) over a real `AssetServer` — nothing is seeded, nothing re-implements
//! the loader.
//!
//! # Standalone include (the `gate.rs` convention, for the opposite reason)
//!
//! This file is deliberately NOT wired into `load_suite/mod.rs`: `dead_code`
//! is live in test crates, so every existing family wrapper that includes
//! `mod load_suite;` without invoking these four drivers would go red. A
//! wrapper opts in with `#[path = "load_suite/behaviors.rs"] mod behaviors;`
//! and implements [`FamilyBehaviorContract`] for its marker. The file is
//! self-contained (no `super::` references, no dependency on the tier suite),
//! which is also why the two registry projections it needs are re-declared on
//! its own trait rather than borrowed from `FamilyLoadContract`.
//!
//! VALUE-AGNOSTIC throughout: presence, authored member KEYS, and an in-memory
//! sentinel edit — never a shipped magnitude (the brittle-test rule).

use std::path::PathBuf;

use bevy::asset::{AssetServer, Assets};
use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_assets::{
    ContentFamily, ContentFinding, ContentIntegrityReport, ContentValidationDone, RonAsset,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async load resolving — a signal-poll cap, never a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// How many updates the never-publish-partial walk holds the world open while
/// a member is knocked out of its collection — a true, small frame count (the
/// still-alive resolve retries every frame; publishing at ANY point is the
/// regression).
const HOLDBACK_UPDATES: u32 = 4;

/// The per-family parameters of the deep-behavior suite: the family's
/// [`ContentFamily`] marker plus the fixtures and value-agnostic hooks the
/// four drivers assert through. Implemented by an opting-in wrapper file for
/// its `gdtf_content_families` marker (the impl lives IN the wrapper, so the
/// orphan rule is satisfied and family N+1 never edits a shared file).
pub(crate) trait FamilyBehaviorContract: ContentFamily {
    /// The fixture-root dir (under `tests/fixtures/`) whose family folder
    /// carries well-formed members BESIDE one deliberately-malformed file —
    /// the salvage-parity scenario.
    const SALVAGE_FIXTURE_ROOT: &'static str;

    /// The authored stems of the WELL-FORMED salvage-fixture siblings — each
    /// must survive the per-file salvage.
    const SALVAGE_GOOD_MEMBERS: &'static [&'static str];

    /// The malformed salvage-fixture member's FILE name (with extension) —
    /// asserted on the [`ContentFinding::MalformedFile`] report entry.
    const SALVAGE_BROKEN_FILE: &'static str;

    /// A fixture-root dir (under `tests/fixtures/`) that does NOT materialize
    /// the family's folder at all — the genuine-`Failed`, fail-closed-EMPTY
    /// scenario (`missing_family_root` is the shared family-agnostic root).
    const MISSING_FOLDER_FIXTURE_ROOT: &'static str;

    /// A SHIPPED member stem (addressable as
    /// `<FOLDER>/<PROBE_MEMBER>.<EXTENSION>`) the never-publish-partial and
    /// redrive walks knock out / hot-edit. Stem-addressable, so this suite
    /// currently fits stem-keyed families.
    const PROBE_MEMBER: &'static str;

    /// True when the resolved registry holds no members.
    fn is_empty(registry: &Self::Registry) -> bool;

    /// Whether the member behind `label` resolves in `registry`.
    fn member_resolves(registry: &Self::Registry, label: &str) -> bool;

    /// Hot-edit one loaded member spec IN MEMORY (the file-watcher stand-in) —
    /// set a sentinel a rebuild makes visible, never a shipped magnitude.
    fn mutate_spec(spec: &mut Self::Spec);

    /// Whether `registry` reflects the [`mutate_spec`](Self::mutate_spec)
    /// sentinel for the member behind `label` — proves the redrive rebuilt.
    fn mutation_visible(registry: &Self::Registry, label: &str) -> bool;
}

/// SALVAGE PARITY (GTW-582 C4 through the generic loader) — a family folder
/// carrying one malformed `.ron` beside well-formed siblings still resolves a
/// registry holding every sibling; the malformed file alone fails, loudly, as
/// a [`ContentFinding::MalformedFile`] naming the family; and `Load` still
/// releases.
pub(crate) fn salvage_parity<F: FamilyBehaviorContract>() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(fixture_root(F::SALVAGE_FIXTURE_ROOT))
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the registry arrives via the per-file salvage path (the
    // folder's own load_folder walk FAILED on the malformed member).
    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);
    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "the broken `{}` folder must still resolve a {} (per-file salvage); last AppState \
         was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        for member in F::SALVAGE_GOOD_MEMBERS {
            assert!(
                F::member_resolves(registry, member),
                "the well-formed sibling `{member}` must be in the salvaged {} (fail-closed \
                 per FILE, not per family)",
                registry_name::<F>(),
            );
        }
        assert!(
            !F::is_empty(registry),
            "one malformed file may not EMPTY the {} (the pre-GTW-582 behavior)",
            registry_name::<F>(),
        );
        // The malformed member alone is missing from the registry.
        let broken_stem = F::SALVAGE_BROKEN_FILE
            .strip_suffix(&format!(".{}", F::EXTENSION))
            .unwrap_or(F::SALVAGE_BROKEN_FILE);
        assert!(
            !F::member_resolves(registry, broken_stem),
            "the malformed member `{broken_stem}` must NOT resolve into the {}",
            registry_name::<F>(),
        );
    }

    // The malformed member is LOUD: a MalformedFile finding names it + the family.
    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
    let reported = app
        .world()
        .get_resource::<ContentIntegrityReport>()
        .is_some_and(|report| {
            report.findings().iter().any(|finding| {
                matches!(
                    finding,
                    ContentFinding::MalformedFile { path, family, .. }
                        if path.contains(F::SALVAGE_BROKEN_FILE)
                            && **family == registry_name::<F>()
                )
            })
        });
    assert!(
        reported,
        "the malformed `{}` must be reported as a MalformedFile finding against the {}; \
         findings: {:?}",
        F::SALVAGE_BROKEN_FILE,
        registry_name::<F>(),
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .map(ContentIntegrityReport::findings),
    );

    assert_load_releases::<F>(&mut app, "the salvaged folder");
}

/// FAIL-CLOSED-EMPTY — a family folder that cannot be enumerated at all (the
/// directory does not exist in the fixture root) publishes the EMPTY registry
/// (ADR-0003), and the presence-gated `Load` flow still releases (the
/// no-strand guarantee).
pub(crate) fn missing_folder_fails_closed_empty<F: FamilyBehaviorContract>() {
    let mut app =
        GdtfLoadTestAppBuilder::with_asset_root(fixture_root(F::MISSING_FOLDER_FIXTURE_ROOT))
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);
    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "a missing `{}` folder must still publish a {} (fail-closed, never a strand); last \
         AppState was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        assert!(
            F::is_empty(registry),
            "an unenumerable `{}` folder must fail closed to the EMPTY {}",
            F::FOLDER,
            registry_name::<F>(),
        );
    }

    assert_load_releases::<F>(&mut app, "the missing folder's empty registry");
}

/// NEVER-PUBLISH-PARTIAL — with one member knocked out of its `Assets`
/// collection (the folder's load state stays `Loaded`), the re-armed resolve
/// publishes NOTHING for the whole holdback; once the member returns, the
/// retry publishes the FULL registry with the member resolving.
pub(crate) fn never_publishes_partial<F: FamilyBehaviorContract>() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);
    assert!(
        app.world().get_resource::<F::Registry>().is_some(),
        "the real `{}` folder must resolve a {} before the partial-hold walk; last AppState \
         was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );

    // Re-arm the absence-gated resolve, then knock ONE member out of its
    // collection (the folder handle + its load state are untouched, so only
    // the never-publish-partial walk can hold the registry back).
    app.world_mut().remove_resource::<F::Registry>();
    let member = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<F::Spec>>(member_path::<F>());
    let removed = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<F::Spec>>>()
        .remove(member.id());
    assert!(
        removed.is_some(),
        "the `{}` member must have been resident to remove",
        member_path::<F>(),
    );

    for _ in 0..HOLDBACK_UPDATES {
        app.update();
    }
    assert!(
        app.world().get_resource::<F::Registry>().is_none(),
        "with `{}` missing from its collection, NOTHING may be published (no partial {})",
        member_path::<F>(),
        registry_name::<F>(),
    );

    // The member returns — the still-alive resolve retries and publishes the
    // FULL registry.
    let Some(spec) = removed else { return };
    let reinserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<F::Spec>>>()
        .insert(member.id(), spec);
    assert!(reinserted.is_ok(), "re-inserting the member must succeed");
    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);
    let resolves = app
        .world()
        .get_resource::<F::Registry>()
        .is_some_and(|registry| F::member_resolves(registry, F::PROBE_MEMBER));
    assert!(
        resolves,
        "once `{}` returns, the retrying resolve must publish the FULL {} (member resolving)",
        member_path::<F>(),
        registry_name::<F>(),
    );
}

/// REDRIVE — a member `Modified` event (queued by an in-place `get_mut` edit,
/// the file-watcher stand-in) rebuilds the resident registry IN PLACE from the
/// latest in-memory specs, through the real registered redrive.
pub(crate) fn redrive_rebuilds_live<F: FamilyBehaviorContract>() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<F::Registry>(&mut app, LOAD_SAFETY_NET);

    // Hot-edit the probe member's payload in place; `get_mut` queues the
    // Modified event the ungated redrive reacts to.
    let member = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<F::Spec>>(member_path::<F>());
    let mutated = {
        let mut specs = app.world_mut().resource_mut::<Assets<RonAsset<F::Spec>>>();
        match specs.get_mut(&member) {
            Some(mut asset) => {
                // Two derefs: through the `AssetMut` guard to the `RonAsset`,
                // then through its payload deref to the spec itself.
                F::mutate_spec(&mut **asset);
                true
            }
            None => false,
        }
    };
    assert!(
        mutated,
        "the `{}` member must be resident to hot-edit",
        member_path::<F>(),
    );

    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<F::Registry>()
                .is_some_and(|registry| F::mutation_visible(registry, F::PROBE_MEMBER))
        },
        LOAD_SAFETY_NET,
    );
    assert!(
        rebuilt,
        "a Modified `{}` member must rebuild the {} in place with the edited spec",
        member_path::<F>(),
        registry_name::<F>(),
    );
}

/// Load must always release past the scenario (Intro is TRANSIENT — probe via
/// `load_released`, never `== Intro`; GTW-589/GTW-601).
fn assert_load_releases<F: FamilyBehaviorContract>(app: &mut bevy::app::App, scenario: &str) {
    let released = advance_until(app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "Load must still release to Intro (or beyond) past {scenario} (`{}`); last AppState \
         was {:?}",
        F::FOLDER,
        app_state(app),
    );
}

/// The family registry's short type name, for assertion messages.
fn registry_name<F: FamilyBehaviorContract>() -> &'static str {
    let full = std::any::type_name::<F::Registry>();
    full.rsplit("::").next().unwrap_or(full)
}

/// A fixture root under this crate's `tests/fixtures/` (the GTW-580 fixture
/// convention: a root materializes ONLY the subdir its scenario overrides).
fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// The probe member's asset path: `<FOLDER>/<PROBE_MEMBER>.<EXTENSION>`.
fn member_path<F: FamilyBehaviorContract>() -> String {
    format!("{}/{}.{}", F::FOLDER, F::PROBE_MEMBER, F::EXTENSION)
}

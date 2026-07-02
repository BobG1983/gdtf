//! GTW-549 (PHASE 1): `AppState::Load` preloads the `assets/content/attachments/` folder
//! through the `RonAsset<AttachmentSpec>` loader (guarded for headless), builds a name-keyed
//! [`AttachmentRegistry`] from the loaded attachment files (keyed by filename stem, minus the
//! `.attachment` infix), and gates the Load->Intro transition on it — the attachment mirror of
//! the weapons load-and-build path ([`load_weapons`](../load_weapons.rs)).
//!
//! Tier (b) — `DefaultPlugins` (headless, `backends: None`) via [`GdtfLoadTestAppBuilder`]: a
//! real `AssetServer` pointed at the workspace `assets/`. The good path loads
//! `assets/content/attachments/*.attachment.ron` into an [`AttachmentRegistry`] keyed by file
//! stem. This proves the SHIPPED attachment items PARSE from RON through the real folder-load
//! path (the "attachment items parse from RON, folder-loaded" verification clause), and that
//! the KEYS authored on the two shipped weapons resolve to their loaded specs.
//!
//! Pin-discriminating + VALUE-AGNOSTIC (mirroring
//! [`real_asset_resolves_weapon_registry_keyed_by_filename`](../load_weapons.rs)): it asserts
//! only the registry's PRESENCE / non-emptiness and that the authored attachment KEYS resolve
//! to NON-EMPTY effect lists — never a shipped magnitude (the brittle-test rule). So a
//! tuning/magnitude edit never reddens it, but a malformed shipped attachment RON, a
//! variant-name typo in a shipped file, a stem-strip regression, or a bad weapon slot key does.
//! The effect-to-stat application MECHANISM is covered by the sim apply/spawn tests
//! (`attachment_item::apply::test` + `gtw549_attachments`); this harness proves the REAL
//! `assets/content/attachments/` folder loads through the Load code path.

use bevy::state::state::State;
use gdtf_app::test_support::AppState;
use gdtf_battle_sim::weapon::{AttachmentName, AttachmentRegistry, WeaponName, WeaponRegistry};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until_resource_exists` wait gated on the
/// async attachments folder load resolving. The load shares the `AssetServer` with the theme /
/// situation / tuning / weapons + the presenter's startup tile-sheet loads (the full scene
/// stack is registered in this harness), so under parallel `cargo` contention the async resolve
/// has NO fixed frame count. The wait keys off the resolved SIGNAL; the cap is a safety net
/// against a genuine never-resolve hang, not a timing budget (GTW-305, mirroring `load_weapons`).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// GTW-549 (tier b) — with a real `AssetServer` rooted at the workspace `assets/`, entering
/// `Load` loads `assets/content/attachments/*.attachment.ron` and builds an
/// [`AttachmentRegistry`] keyed by each file's stem. Proves the SHIPPED attachment items PARSE
/// from RON through the folder-load code path (non-empty registry), and that the KEYS the two
/// shipped weapons authored (`scoped_sight` on `las_carbine`, `suppressor` on `stub_pistol`)
/// resolve — both to the loaded attachment spec AND, transitively, that each weapon's authored
/// slot key resolves to a NON-EMPTY effect list in the registry (so the live weapon references
/// don't resolve to nothing at runtime).
///
/// This mirrors the weapons precedent
/// [`real_asset_resolves_weapon_registry_keyed_by_filename`](../load_weapons.rs). It does NOT
/// pin any authored magnitude — those are per-item DATA (the brittle-test rule); it asserts
/// only presence + non-empty effect lists.
#[test]
fn real_asset_resolves_attachment_registry_and_shipped_weapon_keys() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async attachments folder load: wait until the AttachmentRegistry is
    // inserted (the resolver only inserts it ONCE fully built from the folder — it stays ABSENT
    // while members are still resolving — so on the good path existence implies the shipped
    // attachments parsed and are present), not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<AttachmentRegistry>(&mut app, LOAD_SAFETY_NET);

    // Also wait for the WeaponRegistry so the shipped weapon slot keys are available to resolve.
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    // Both registries must have resolved from the workspace assets/. Assert PRESENCE loudly
    // first (pin-discriminating), THEN bind without a panic (restriction lints deny `panic!`
    // even in tests) — the `is_some` assert already failed loudly if either is absent.
    assert!(
        app.world().get_resource::<AttachmentRegistry>().is_some()
            && app.world().get_resource::<WeaponRegistry>().is_some(),
        "both an AttachmentRegistry and a WeaponRegistry must resolve from the workspace \
         assets/ (last AppState was {:?})",
        app_state(&app),
    );
    let (Some(attachments), Some(weapons)) = (
        app.world().get_resource::<AttachmentRegistry>(),
        app.world().get_resource::<WeaponRegistry>(),
    ) else {
        return;
    };

    // The shipped attachment items PARSED and populated the registry (the folder-loaded parse
    // clause) — a malformed / variant-typo'd shipped RON would leave this empty or partial.
    assert!(
        !attachments.is_empty(),
        "the resolved AttachmentRegistry must carry the shipped (non-empty) attachment items \
         — a bad/malformed shipped `*.attachment.ron` would leave it empty",
    );

    // The two shipped weapons authored these attachment KEYS; each must resolve to its loaded
    // attachment spec, and that spec's effect list must be NON-EMPTY (both are functional
    // attachments — a sight and a suppressor) so the live weapon references don't resolve to
    // nothing. Value-agnostic: presence + non-empty, never a magnitude.
    for (weapon_key, attachment_key) in [
        ("las_carbine", "scoped_sight"),
        ("stub_pistol", "suppressor"),
    ] {
        let name = AttachmentName::new(attachment_key.to_owned());

        // The shipped weapon RON references the attachment slot KEY.
        let weapon_authors_key = weapons
            .spec(&WeaponName::new(weapon_key.to_owned()))
            .is_some_and(|weapon| weapon.attachments.contains(&name));
        assert!(
            weapon_authors_key,
            "the shipped weapon `{weapon_key}` must resolve and author the `{attachment_key}` \
             attachment slot key (the shipped weapon RON references it)",
        );

        // The key resolves to a loaded attachment spec with a NON-EMPTY effect list — a
        // stem-strip regression, a missing shipped file, or an effect-less item breaks this.
        let key_resolves_to_effects = attachments
            .spec(&name)
            .is_some_and(|spec| !spec.effects.is_empty());
        assert!(
            key_resolves_to_effects,
            "the `{attachment_key}` key authored on `{weapon_key}` must resolve to a loaded \
             attachment spec with a non-empty effect list (not resolve to nothing at runtime)",
        );
    }
}

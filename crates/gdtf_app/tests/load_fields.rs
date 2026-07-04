//! GTW-545 (area-damage fields, child GTW-41f): `AppState::Load` preloads the
//! `assets/content/fields/` folder through the `RonAsset<FieldDef>` loader (guarded for
//! headless), builds a stem-keyed `FieldDefRegistry` catalog from the loaded field files
//! (keyed by filename stem, `.field` infix stripped), and gates the Load->Intro transition on
//! it — so the machine never leaves `Load` before the fields folder is verified loaded. This
//! mirrors the GTW-269 armor load-and-build path exactly.
//!
//! Two tiers (mirroring the armor harness split, `load_armor.rs`):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no `AssetServer`, so
//!   the field-loader registration + folder kick-off must no-op without panicking. Injecting
//!   the full gate resource set (including an empty `FieldDefRegistry`) drives the real gated
//!   transition; the app advances past `Load`. A companion negative test WITHHOLDS the
//!   `FieldDefRegistry` and asserts the machine stays in `Load`.
//! - **Tier (b)** — `DefaultPlugins` (headless) via [`GdtfLoadTestAppBuilder`]: a real
//!   `AssetServer` pointed at the workspace `assets/`. The good path loads
//!   `assets/content/fields/*.field.ron` into a `FieldDefRegistry` keyed by file stem
//!   (`toxic_waste_pool`, `electrified_floor`, `incendiary_fire`).
//!
//! These are *pin-discriminating* + VALUE-AGNOSTIC (only the catalog's PRESENCE + that it is
//! keyed by the authored filename stems), so an authored-magnitude edit never reddens them (the
//! brittle-test rule). The per-round drain / immunity / seed MECHANISM is covered by the
//! in-crate sim tests + the `gtw545_fields` integration test; this harness proves the REAL
//! folder loads through the Load code path and that its authored KEYS resolve.

use gdtf_app::test_support::{AppState, LoadedSituation, app_state};
use gdtf_battle_sim::{
    FieldDefRegistry, FieldKey,
    armor::ArmorRegistry,
    injuries::InjuryRegistry,
    procgen::ProcgenTuning,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::default_theme;

/// Bounded budget for the Tier (a) `MinimalPlugins` transition / negative waits, where all gate
/// resources are injected by hand — a true, small, deterministic frame count.
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits gated on an async
/// asset load resolving (the fields folder shares the `AssetServer` with the whole scene
/// stack). These waits key off the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (the GTW-305 idiom).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Seed the full Load gate resource set EXCEPT the [`FieldDefRegistry`] — the other
/// gate-blocking resources stand in for their asset-less resolves, so the ONLY thing that can
/// hold or release the gate in a test is the field registry the caller adds (or withholds).
fn seed_gate_except_fields(app: &mut bevy::app::App) {
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(ProcgenTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
}

/// AC (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering `Load` must not
/// panic: the field-loader registration + `load_folder("content/fields")` kick-off both guard
/// on a missing server and no-op. The machine still advances past `Load` once the full gate set
/// (including an empty `FieldDefRegistry`) is injected.
#[test]
fn field_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    seed_gate_except_fields(&mut app);
    // GTW-545: the FieldDefRegistry is the last gate resource — with it present, Load advances.
    app.world_mut().insert_resource(FieldDefRegistry::default());

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with the full gate set (incl. an empty FieldDefRegistry) present, Load must advance to \
         Intro within {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// AC (companion, tier a) — the Load->Intro transition GATES on the `FieldDefRegistry`: with
/// every other gate resource present but NO field catalog (and no `AssetServer` to resolve
/// one), the machine must stay in `Load` for the whole budget. Proves the field catalog is a
/// genuine gate-blocking resource (the machine never leaves `Load` before the fields folder is
/// verified loaded — else a battle seeding a field could hit `FieldNotFound` at setup).
#[test]
fn load_does_not_leave_without_a_field_def_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Every other gate resource present, but the FieldDefRegistry deliberately withheld.
    seed_gate_except_fields(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the FieldDefRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no FieldDefRegistry present, the machine stays in Load (the fields folder must be \
         verified loaded before Load exits)",
    );
}

/// AC (tier b) — with a real `AssetServer` rooted at the workspace `assets/`, entering `Load`
/// loads `assets/content/fields/*.field.ron` and builds a `FieldDefRegistry` keyed by each
/// field file's filename stem (with the `.field` infix stripped). Proves the folder loaded into
/// the catalog keyed by filename (the authored `toxic_waste_pool` key resolves), and that the
/// Load gate waited for it (the machine reaches Intro with a catalog present). VALUE-AGNOSTIC —
/// no authored magnitude is pinned.
#[test]
fn real_asset_resolves_field_def_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async fields folder load: wait until the FieldDefRegistry is inserted.
    advance_until_resource_exists::<FieldDefRegistry>(&mut app, LOAD_SAFETY_NET);

    // The catalog is keyed by the authored filename stems (with the `.field` infix stripped):
    // `toxic_waste_pool.field.ron` keys `toxic_waste_pool`. Each authored key resolves to its
    // loaded def.
    if let Some(registry) = app.world().get_resource::<FieldDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved FieldDefRegistry must carry the authored (non-empty) field types",
        );
        assert!(
            registry
                .def(&FieldKey::new("toxic_waste_pool".to_owned()))
                .is_some(),
            "the catalog must hold `toxic_waste_pool` (keyed by toxic_waste_pool.field.ron's stem)",
        );
    }

    // The Load gate WAITED for the catalog: the machine reaches Intro, with a FieldDefRegistry
    // present when it does (the GTW-545 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once the fields folder resolves; last \
         AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<FieldDefRegistry>().is_some(),
        "a FieldDefRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
}

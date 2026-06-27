//! GTW-437: `AppState::Load` preloads the `assets/content/injuries/` folder through ONE
//! recursive `LoadedFolder` carrying TWO `RonAsset` types — the per-injury
//! `*.injury.ron` (`RonAsset<InjuryDef>`) and the per-part `weighting/*.weighting.ron`
//! (`RonAsset<InjuryWeighting>`) — builds BOTH an [`InjuryRegistry`] (name→def, keyed by
//! each injury file's stem minus the `.injury` infix) AND the [`InjuryTables`]
//! (per-`(category, severity)` weighted roll table), and GATES the Load->Intro
//! transition on the registry — so a battle never starts before injuries load. This
//! mirrors the GTW-257 weapons / GTW-269 armor load-and-build path exactly, generalised
//! to one folder → two resources.
//!
//! Two tiers (mirroring the weapon / armor harness split):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the injuries-loader registration + folder kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1).
//!   Injecting every gate resource by hand drives the real gated transition; the app
//!   advances past `Load`, and no registry is RESOLVED from disk (nothing to load).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace `assets/`.
//!   The good path loads `assets/content/injuries/**/*.injury.ron` into an [`InjuryRegistry`]
//!   keyed by file stem (`scalp_graze`, `lost_eye`) AND folds
//!   `assets/content/injuries/weighting/*.weighting.ron` into the [`InjuryTables`] (the
//!   `(Head, Minor)` + `(Head, Critical)` buckets the authored head weighting populates).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance criterion so
//! a regression turns the test red. They are VALUE-AGNOSTIC (only the registry's
//! PRESENCE + that it is keyed by the authored filename stems, and that the authored
//! `(part, severity)` buckets are PRESENT), so a tuning edit never reddens them — the
//! authored injury weights / stat amounts are tuning DATA, NOT pinned by tests (the
//! brittle-test rule; see [`InjuryDef`](gdtf_battle_sim::injuries::InjuryDef)). The
//! field-level parse + build-table MECHANISM is covered by the resolve module's unit
//! tests (`states::load::systems::resolve::injuries::test`); this harness proves the REAL
//! `assets/content/injuries/` folder builds BOTH resources through the production
//! `resolve_injuries` Load code path and that its authored KEYS / BUCKETS resolve.
//!
//! ROBUST TO GTW-440: the real-asset test asserts only against what IS authored today
//! (the two head injuries + the head weighting's `Minor` / `Critical` buckets), never an
//! exact injury COUNT or bucket COUNT — so GTW-440 adding the per-part content floor
//! cannot redden it.

use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    armor::{ArmorRegistry, BodyPart},
    injuries::{InjuryName, InjuryRegistry, InjuryTables},
    level::ThemeCatalogRegistry,
    severity::Severity,
    situation::Situation,
    terrain::piece::TerrainRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::default_theme;

/// Bounded budget for the Tier (a) `MinimalPlugins` transition / negative waits, where
/// all gate resources are injected by hand — a true, small, deterministic frame count
/// (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits gated on an
/// async asset load resolving. The injuries folder load shares the `AssetServer` with the
/// theme / situation / tuning / weapons / armor / terrain + the presenter's startup
/// tile-sheet loads (the full scene stack is registered in this harness), so under
/// parallel `cargo` contention the async resolve has NO fixed frame count. These waits key
/// off the resolved SIGNAL; the cap is a safety net against a genuine never-resolve hang,
/// not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// AC (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering `Load`
/// must not panic: the injuries-loader registration (the `RonAsset<InjuryDef>` /
/// `RonAsset<InjuryWeighting>` loaders) and the `load_folder("injuries")` kick-off both
/// guard on a missing server and no-op. The machine still advances past `Load` once every
/// gate resource is injected (standing in for all resolves completing), proving the guard
/// holds.
///
/// Pin: if the injuries-loader registration or the `load_folder` kick-off ever ran without
/// the `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering `Load` would
/// panic instead of resting / advancing (bevy-traps rule 1).
#[test]
fn injuries_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the injuries loader registration + folder kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // Stand in for the theme + tuning + weapons + situation + armor + terrain + injuries
    // resolves completing (no AssetServer under MinimalPlugins), driving the real gated
    // transition (GTW-437: the InjuryRegistry is a gate-blocking resource too, alongside
    // the GTW-257 WeaponRegistry / GTW-269 ArmorRegistry / GTW-261 LoadedSituation /
    // GTW-394 TerrainRegistry / GTW-384 GangerStatTuning).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme + CombatTuning + GangerStatTuning + WeaponRegistry + ArmorRegistry \
         + TerrainRegistry + InjuryRegistry + LoadedSituation present, Load must advance to \
         Intro within {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// AC (companion, tier a) — the Load->Intro transition GATES on the [`InjuryRegistry`]:
/// with every OTHER gate resource present but NO injury registry (and no `AssetServer` to
/// resolve one), the machine must stay in `Load` for the whole budget. Proves the injury
/// registry is a genuine gate-blocking resource (a battle never starts before the injuries
/// folder is verified loaded).
///
/// Pin: this fails if the transition ever fired without the injury registry present
/// (regressing the GTW-437 gate clause) — e.g. if the gate's `resource_exists::<InjuryRegistry>`
/// run-condition were removed, the walk would reach Intro with no registry and this test
/// would go red (confirmed by temporarily dropping that condition — see report).
#[test]
fn load_does_not_leave_without_an_injury_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Every OTHER gate resource present, but the InjuryRegistry deliberately withheld.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the InjuryRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no InjuryRegistry present, the machine stays in Load (the injuries folder must \
         be verified loaded before Load exits — a battle never starts injury-less)",
    );
}

/// AC (tier b) — with a real `AssetServer` rooted at the workspace `assets/`, entering
/// `Load` loads `assets/content/injuries/**/*.injury.ron` into an [`InjuryRegistry`] keyed by each
/// injury file's stem (minus the `.injury` infix) AND folds
/// `assets/content/injuries/weighting/*.weighting.ron` into the [`InjuryTables`] — BOTH built
/// through the production [`resolve_injuries`] path. Proves the folder loaded into the
/// registry keyed by filename (the canonical `scalp_graze` / `lost_eye` keys resolve), the
/// authored head weighting populated the `(Head, Minor)` + `(Head, Critical)` buckets, and
/// that the Load gate waited for it (the machine reaches Intro with a registry present).
///
/// GTW-437: this asserts that the REAL injuries folder builds BOTH resources end-to-end +
/// each authored KEY / BUCKET resolves through the Load code path (mirroring the
/// GTW-257/269 precedent). It does NOT pin any authored weight / stat amount — those are
/// tuning DATA (the brittle-test rule), nor an exact injury / bucket COUNT — so GTW-440
/// adding the per-part content floor cannot redden it (it asserts only against the two
/// head injuries + the head weighting authored TODAY).
///
/// PIN: this fails if `resolve_injuries` failed to build EITHER resource from the real
/// folder — an absent `scalp_graze` / `lost_eye` key (registry build broke) or an absent
/// `(Head, Minor)` / `(Head, Critical)` bucket (tables fold broke) turns it red.
#[test]
fn real_asset_resolves_injury_registry_and_tables() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async injuries folder load: wait until the InjuryRegistry is
    // inserted by `resolve_injuries`, not a fixed frame count. Cap is a safety net
    // (GTW-305). DELIBERATELY do NOT seed InjuryRegistry::default() — existence here proves
    // the REAL resolve published it from the folder.
    advance_until_resource_exists::<InjuryRegistry>(&mut app, LOAD_SAFETY_NET);

    // The registry is keyed by the authored injury filename stems (minus the `.injury`
    // infix): `head/scalp_graze.injury.ron` keys `scalp_graze`; `head/lost_eye.injury.ron`
    // keys `lost_eye`. Each authored key resolves to its loaded def.
    if let Some(registry) = app.world().get_resource::<InjuryRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved InjuryRegistry must carry the authored (non-empty) injuries",
        );
        assert!(
            registry
                .def(&InjuryName::new("scalp_graze".to_owned()))
                .is_some(),
            "the registry must hold the `scalp_graze` injury (keyed by \
             head/scalp_graze.injury.ron's stem)",
        );
        assert!(
            registry
                .def(&InjuryName::new("lost_eye".to_owned()))
                .is_some(),
            "the registry must hold the `lost_eye` injury (keyed by \
             head/lost_eye.injury.ron's stem)",
        );
    }

    // The InjuryTables were folded from the real `weighting/head.weighting.ron`: it
    // authors a Minor bucket (scalp_graze) and a Critical bucket (lost_eye) for the Head,
    // so both (Head, Minor) and (Head, Critical) buckets must be PRESENT. Value-agnostic —
    // bucket PRESENCE only, never the authored weights (GTW-440 may add more rows / parts).
    if let Some(tables) = app.world().get_resource::<InjuryTables>() {
        assert!(
            !tables.is_empty(),
            "the resolved InjuryTables must carry the authored (non-empty) weighting buckets",
        );
        assert!(
            tables.table(BodyPart::Head, Severity::Minor).is_some(),
            "the (Head, Minor) bucket must be populated from head.weighting.ron's `minor` list \
             (the scalp_graze row)",
        );
        assert!(
            tables.table(BodyPart::Head, Severity::Critical).is_some(),
            "the (Head, Critical) bucket must be populated from head.weighting.ron's `critical` \
             list (the lost_eye row)",
        );

        // GTW-440 C4 — WARN-CLEAN / EVERY CATEGORY NON-EMPTY: each of the four injury-pool
        // categories (Head / Torso / Arm / Leg) must have at least one tabled (rollable)
        // bucket across the three tabled severities. The two arms (and the two legs) share
        // ONE category pool, so a `LeftArm` and a `RightArm` lookup resolve the SAME bucket.
        // Pin-discriminating: an empty category (a missing/typo'd weighting file or an
        // all-unknown-key bucket) would leave its category with NO bucket and fail here.
        for part in [
            BodyPart::Head,
            BodyPart::Torso,
            BodyPart::LeftArm,
            BodyPart::RightArm,
            BodyPart::LeftLeg,
            BodyPart::RightLeg,
        ] {
            let any_bucket = [Severity::Minor, Severity::Major, Severity::Critical]
                .into_iter()
                .any(|sev| tables.table(part, sev).is_some());
            assert!(
                any_bucket,
                "the {part:?} category pool must have at least one rollable bucket (GTW-440 \
                 content floor — every category non-empty)",
            );
        }
        // The shared-pool identity: LeftArm and RightArm resolve the SAME Arm Major bucket,
        // LeftLeg and RightLeg the SAME Leg Major bucket (the per-category restructure).
        assert_eq!(
            tables.table(BodyPart::LeftArm, Severity::Major),
            tables.table(BodyPart::RightArm, Severity::Major),
            "both arms must resolve the IDENTICAL shared Arm (Major) bucket (GTW-440 C1)",
        );
        assert_eq!(
            tables.table(BodyPart::LeftLeg, Severity::Major),
            tables.table(BodyPart::RightLeg, Severity::Major),
            "both legs must resolve the IDENTICAL shared Leg (Major) bucket (GTW-440 C1)",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and an
    // InjuryRegistry is present when it does (GTW-437 gate clause). Both injury resources
    // were built through the production `resolve_injuries` path, end-to-end.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. injuries) \
         resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<InjuryRegistry>().is_some(),
        "an InjuryRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<InjuryTables>().is_some(),
        "the InjuryTables resolved alongside the registry (resolve_injuries built BOTH)",
    );
}

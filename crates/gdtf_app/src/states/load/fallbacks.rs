//! The BESPOKE headless `Load` fallbacks — Load-orchestration policy (GTW-629).
//!
//! Fallback-when-headless is the `Load` scene's concern, not a QA-drive
//! behavior, so the seed system lives here with the plugin that owns the
//! resolves it stands in for. TEST-ONLY (GTW-749): the dev-only `AutoBattlePlugin`
//! was this system's one production registrar, calling it on `Startup` to let a
//! debug GUI launch reach a battle without a real content pipeline; GTW-749
//! retired that affordance outright (the network `StartBattle` path replaces its
//! boot-into-battle role), so [`seed_load_fallbacks`] has no production caller
//! left — only the headless integration tests below that register it on their
//! own `Startup` schedule, and [`crate::test_support::seed_load_gate`]'s wrapper.
//!
//! Only the BESPOKE loads are seeded here (the injuries pair, the prefab
//! registry, the situation, the tunings, the theme). The eight generic content
//! families (weapons / melee / armor / fields / gangs / terrain + theme defs /
//! attachments) get their headless fallback from their ONE
//! `register_content_family` line — the registration seeds `Registry::default()` when
//! there is no [`AssetServer`] (the GTW-629 rider; the shadow-avoidance
//! invariant is stated once, at that registration), so this file carries ZERO
//! content-family fallback arms.

#[cfg(feature = "test-support")]
use bevy::prelude::*;
#[cfg(feature = "test-support")]
use gdtf_battle_sim::{
    injuries::{InjuryRegistry, InjuryTables},
    level::PrefabRegistry,
    procgen::ProcgenTuning,
    tuning::CombatTuning,
};
#[cfg(feature = "test-support")]
use gdtf_ui::theme::default_theme;

#[cfg(feature = "test-support")]
use crate::states::load::resources::LoadedSituation;

/// Seeds the persistent BESPOKE `Load` resources a headless (asset-less) walk
/// needs so the state machine can traverse `Load` without a resolved asset
/// stack.
///
/// Inserts [`default_theme`] + [`CombatTuning::default`] UNGATED, exactly as the
/// retired auto-battle seeder did. KNOWN DORMANT DEFECT (pre-existing; surfaced at
/// the GTW-629 gate — the fix belongs to its own bug ticket, not this
/// behavior-preserving move; GTW-749 retired this system's only production
/// caller, so the defect can no longer surface in a real launch, only in a test
/// that calls this UNGATED path against a real `AssetServer`): under a
/// real-`AssetServer` build these two seeds SHADOW the shipped
/// `ui_theme.tuning.ron` / `combat.tuning.ron`, because the theme resolve skips
/// once a `GdtfTheme` exists (`resolve/poll.rs`), the tuning resolve is
/// absence-gated (the GTW-564 hot-RON chain), and the hot-RON redrive fires on
/// `AssetEvent::Modified` only — the same `AC3b` shadow class the gated arms
/// below exist to avoid.
///
/// **The A1 / GTW-297 `AC3b` seed-shadow invariant (once, for every gated arm
/// below):** each remaining bespoke resolve — the injuries pair (GTW-437/438),
/// the UUID-keyed prefab registry (GTW-489), the procgen tuning (GTW-533), and
/// the authored situation (GTW-261) — only RUNS while its resource is ABSENT,
/// so a pre-seeded default would shadow the real asset (the `AC3b`
/// `WeaponNotFound` black-screen class, and the A1 empty-battlefield race).
/// Every gated arm therefore seeds ONLY when no [`AssetServer`] is present (a
/// headless / asset-less build); with one present the real resolves win.
///
/// The eight generic content families carry NO arm here: their headless
/// fallback rides `register_content_family` itself (the GTW-629 rider — see
/// `gdtf_assets`' `ContentFamilyAppExt`), so a new folder family needs its one
/// registration line and nothing else.
///
/// Runs once in `Startup` (before the first `Update`, hence before `Load`
/// resolves). Param-only (`bevy-traps.md` #7): [`Commands`] + an
/// `Option<Res<AssetServer>>` probe (`Option` so it is panic-free whether or
/// not the asset stack is wired) — no `&mut World`.
///
/// TEST-ONLY (GTW-749): `pub` under `test-support` only — every remaining
/// caller is a headless integration test registering it on its own `Startup`
/// schedule (or `crate::test_support::seed_load_gate`'s wrapper), never a
/// `pub(crate)` production path.
#[cfg(feature = "test-support")]
pub fn seed_load_fallbacks(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    commands.insert_resource(default_theme());
    commands.insert_resource(CombatTuning::default());
    if asset_server.is_none() {
        // GTW-437: the InjuryRegistry is a gate-blocking resource; the injuries
        // folder is a BESPOKE load (one folder, two resources).
        commands.insert_resource(InjuryRegistry::default());
        // GTW-438: the InjuryTables is read by the fire path's `roll_injury` (the
        // first reader) — seeded alongside the registry for parity with the
        // resolve path, else `dispatch_fire`'s `Res<InjuryTables>` would panic on
        // a missing resource in an asset-less headless drive. An empty table
        // means the roll finds no bucket and still takes-then-discards its draw.
        commands.insert_resource(InjuryTables::default());
        // GTW-489: the UUID-keyed PrefabRegistry is gate-blocking; the maps tree
        // is a BESPOKE load (UUID multimap). GTW-494: this is the ONLY prefab
        // registry.
        commands.insert_resource(PrefabRegistry::default());
        // GTW-533: the ProcgenTuning is gate-blocking; seed the const RULED
        // default.
        commands.insert_resource(ProcgenTuning::default());
        // GTW-261: a present LoadedSituation SATISFIES the Load→Intro gate, so
        // the EMPTY battlefield is seeded only on the asset-less path.
        commands.insert_resource(LoadedSituation::new(
            gdtf_battle_sim::situation::Situation::default(),
        ));
    }
}

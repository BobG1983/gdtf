//! Shared `fx_draw` fixture: the headless renderer app, async-load settling,
//! deterministic time stepping, and impact-signal draining.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    CharacterRoles, EffectRoles, FxTuning, Played, ShotImpactResolved, TopDownAtlases,
    TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    acts::{MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::{bleed::Bleeding, dot::DotTicked},
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
};
use gdtf_test_utils::advance_until_resource_exists;

/// Generous SAFETY-NET cap for the async atlas / effect-role / tuning / character
/// loads. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed
/// frame count), which is what makes these FX tests deterministic under parallel
/// `cargo` load (GTW-305). A fixed 128-update budget previously starved under
/// contention and silently left the `run_if`-gated spawn systems no-op (0 vs 1
/// projectile).
pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

/// The workspace-root `assets/` directory (this crate's manifest → up two → assets),
/// the same root the running app uses so the shipped sheets + `effect_roles.ron` load.
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`
/// (workspace `assets/`), the `TopDownRendererPlugin`, and the three sim FX message buffers
/// the readers drain. It does NOT add the sim's lifecycle systems — the test authors the
/// gangers + `BattleInProgress` directly and writes the FX messages itself.
pub(crate) fn headless_renderer_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    // The FX readers drain these buffers; the sim's plugins register them in the real app,
    // but this focused harness adds only the ones the readers need (the GTW-290 ShotFired
    // included).
    .add_message::<Bleeding>()
    .add_message::<ArmorBroken>()
    .add_message::<CoverDestroyed>()
    .add_message::<ShotFired>()
    // GTW-526 C8: the suppression FCT reader drains this buffer; the sim's acts plugin
    // registers it in a real battle, but this focused presenter-only harness adds it itself
    // (matching the Bleeding / ArmorBroken buffers above — the reader is `run_if`-gated on the
    // buffer's presence, so it would otherwise stay inert).
    .add_message::<SuppressionApplied>()
    // GTW-544: the DOT FCT reader drains this buffer; the sim's acts plugin registers it in a real
    // battle, but this focused presenter-only harness adds it itself (matching the buffers above —
    // the reader is `run_if`-gated on the buffer's presence, so it would otherwise stay inert).
    .add_message::<DotTicked>()
    // GTW-623 C5: the melee-strike / fall-impact / grenade-blast readers drain these three
    // sim-owned buffers. The presenter's registrar no longer `add_message`s them itself
    // (GTW-623 C4 — the FieldTicked / OnDeathOccurred precedent): the sim's plugins register
    // them in a real battle, so this focused presenter-only harness seeds the ones its tests
    // write (each reader is `run_if`-gated on its buffer's presence and would otherwise stay
    // inert, silently dropping the written message).
    .add_message::<MeleeResolved>()
    .add_message::<FallOccurred>()
    .add_message::<ThrowResolved>()
    .add_plugins(TopDownRendererPlugin);
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); 0.18 silently SKIPPED. This no-renderer harness lacks the
    // render-provided resources some DefaultPlugins systems want (e.g. bevy_light's
    // update_gizmo_meshes -> Assets<GizmoAsset>), so `warn` restores the 0.18 skip
    // behavior instead of an intermittent headless panic.
    app.set_error_handler(warn);
    app
}

/// PLAYS `fact` — writes it onto the `Played<M>` buffer the PACED FX readers drain.
///
/// Since GTW-727 C16 the six flash-family readers and `spawn_shot_projectiles` fire when the
/// presenter SHOWS a fact, not when the sim produced it, so they read `Played<M>` and the
/// playback cursor is the only thing that writes one. A focused FX test therefore hands its
/// fact to the same buffer the cursor would: the reader under test, its gate, and every
/// assertion about what it draws are unchanged — only the buffer the fact arrives on moves.
///
/// Since GTW-889 the CONSEQUENCE-FCT family readers (`read_consequence_fct::<C>`) drain
/// `Played<C::Signal>` too, so every consequence-FCT test hands its fact to this helper as
/// well. They previously wrote the raw sim buffer, on the belief that their pops were
/// already paced by the projectile → `PendingImpact` → `animate_impact` pipeline. That was
/// wrong: a pop is spawned from the SIGNAL, not from the impact, so an unpaced buffer put
/// the "SUPPRESSED" tag and the injury names on screen at sim time — ahead of the shots
/// that caused them, the GTW-889 symptom.
///
/// The write is ASSERTED rather than ignored: `World::write_message` merely logs and returns
/// `None` on an unregistered buffer, which would read here as "the FX never drew" — a wiring
/// failure wearing a behavioural failure's clothes.
pub(crate) fn play<M: bevy::ecs::message::Message + Clone>(app: &mut App, fact: M) {
    let written = app.world_mut().write_message(Played::new(fact)).is_some();
    assert!(
        written,
        "the Played<{}> buffer must be registered by the presenter's playback registration",
        core::any::type_name::<M>(),
    );
}

/// Drives `update()`s until `EffectRoles` + `TopDownAtlases` + `FxTuning` + `CharacterRoles`
/// are ALL resident (the async load chain has settled), polling each resource's inserted
/// SIGNAL rather than a fixed frame count (GTW-305). All four resolve over the same async
/// `AssetServer` chain, so waiting for them in sequence drives the app until the LAST one
/// is present. Panics (naming the missing resource) if any is still absent after the
/// safety-net cap — a genuine load failure, surfaced loudly rather than silently leaving the
/// `run_if`-gated spawn systems no-op.
///
/// `FxTuning` (GTW-306) is part of the settle gate because `spawn_shot_projectiles` now
/// `run_if(resource_exists::<FxTuning>)` — without waiting for it the projectile-spawn tests
/// flake (the system silently does not run until the hot-reloadable tuning has resolved).
/// `CharacterRoles` is part of the gate so the entity-aim test's real `spawn_ganger_sprites`
/// path (gated `run_if(resource_exists::<CharacterRoles>)`) actually runs and registers the
/// hit ganger's presenter sprite in `GangerSprites` — the lookup the entity-aim branch reads.
pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<EffectRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<FxTuning>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<CharacterRoles>(app, LOAD_SAFETY_NET);
}

/// Reads the resolved `EffectRoles` resource as a clone, or `None` if it is absent (the
/// caller asserts it is `Some` — `settle_resources` already gated on its presence).
pub(crate) fn effect_roles(app: &App) -> Option<EffectRoles> {
    app.world().get_resource::<EffectRoles>().cloned()
}

/// Advances the clock past the flash TTL so `expire_flashes` ticks each live flash past
/// expiry, then restores `Automatic`.
///
/// Sets `TimeUpdateStrategy::ManualDuration(250ms)` (the Bevy test idiom) — 250ms is the
/// virtual clock's default `max_delta` clamp, so a single larger jump would be capped at
/// 250ms anyway. It then runs a SMALL FIXED number of `update()`s (each advancing the virtual
/// `Time` by 250ms, the `delta` `expire_flashes` ticks with), comfortably exceeding the
/// sub-second `FlashTtl` window across the batch. Bounded (no spin), and no new message is
/// written across these updates, so no new flash spawns.
pub(crate) fn advance_past_ttl(app: &mut App) {
    /// The per-update manual delta (== the virtual clock's default `max_delta` clamp).
    const STEP: std::time::Duration = std::time::Duration::from_millis(250);
    /// Enough 250ms steps to clear any sub-second `FlashTtl` window with margin.
    const STEPS: u32 = 8;

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
    for _ in 0..STEPS {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// Runs exactly ONE `update()` with the clock delta pinned to ZERO, then restores `Automatic`
/// time — the firing-FX read frame, made delta-deterministic.
///
/// The firing tests write a `ShotFired` and then read the spawned projectile expecting it AT the
/// muzzle. Under `DefaultPlugins`' default `Automatic` time the first update after the
/// variable-length [`settle_resources`] carries a non-deterministic wall-clock delta, so
/// `advance_projectiles` could move the bolt a contention-dependent distance off the muzzle and
/// flake the assertion (GTW-305, same parallel-`cargo` non-determinism family as the load flakes).
/// A `ManualDuration(0)` delta keeps the bolt exactly where `spawn_shot_projectiles` placed it,
/// regardless of scheduling, without weakening any assertion.
pub(crate) fn fire_with_zero_delta(app: &mut App) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// Steps the app one `step`-sized manual update at a time (up to `max_steps`) until a live FCT
/// pop whose string equals `text` first appears, and returns the `(text, world y)` snapshot of
/// ALL live pops captured on THAT update — the frame the target pop just materialized on its
/// `SpawnScene` schedule, BEFORE `animate_floating_text` next ticks it, so the target pop's `y`
/// is still its unshifted spawn `y` (a direct readout of its stacking slot). Returns `None` if
/// the pop never appears within `max_steps`. Restores `Automatic` time before returning.
///
/// This is the deterministic way to observe a SHOT pop's slot at the moment it spawns: the shot
/// pops ride the projectile → impact pipeline (several fly frames), so a fixed `step_app` count
/// would read them AFTER they have risen. Catching the exact impact frame keeps the read
/// rise-free.
pub(crate) fn step_until_pop(
    app: &mut App,
    text: &str,
    step: std::time::Duration,
    max_steps: u32,
) -> Option<Vec<(String, f32)>> {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut found = None;
    for _ in 0..max_steps {
        app.update();
        let snapshot = super::probes::fct_pops_with_y(app);
        if snapshot.iter().any(|(t, _)| t == text) {
            found = Some(snapshot);
            break;
        }
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    found
}

/// Advances the app a FIXED number of `step`-sized manual updates (each advancing the virtual
/// clock by `step`), then restores `Automatic` time — the deterministic way to fly a staggered
/// volley's bolts to their impacts and watch the per-shot FCT pops appear over time.
pub(crate) fn step_app(app: &mut App, step: std::time::Duration, updates: u32) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..updates {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// Drains and returns the `ShotImpactResolved` signals (GTW-328) emitted SINCE the last drain —
/// the per-shot impact-resolved messages `animate_impact` writes. The combat-text LOG drains this
/// exact buffer to build its shot-outcome lines, so the count of these signals over a stepped run
/// is the number of outcome lines the log gains: zero means no shot has resolved its impact yet.
pub(crate) fn drain_impacts(app: &mut App) -> Vec<ShotImpactResolved> {
    app.world_mut()
        .resource_mut::<Messages<ShotImpactResolved>>()
        .drain()
        .collect()
}

/// Advances the app a fixed number of `step`-sized manual updates, DRAINING the
/// `ShotImpactResolved` buffer after EACH update and accumulating the total emitted across the
/// run, then restores `Automatic` time. The per-update drain is what makes the accumulation
/// reliable: the buffer double-buffers, so a message left un-drained across two updates is
/// dropped — draining each frame captures every staggered impact as it resolves.
pub(crate) fn step_counting_impacts(
    app: &mut App,
    step: std::time::Duration,
    updates: u32,
) -> usize {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut total = 0;
    for _ in 0..updates {
        app.update();
        total += drain_impacts(app).len();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    total
}

//! GTW-572 shared `FctSlotAllocator` + registrar buffer-inertness contract.

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    prelude::{Text2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{FctValence, FloatingCombatText, TopDownRendererPlugin, valence_color};
use gdtf_battle_sim::{
    acts::{InjuryInflicted, MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::{bleed::Bleeding, dot::DotTicked, on_death::OnDeathOccurred},
    falls::FallOccurred,
    occupancy_sync::SlabDestroyed,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    weapon::DotDamage,
};

use super::{harness::*, probes::*};

/// GTW-793 (acceptance 1) — the lifetime-aware allocator: two DIFFERENT consequence families
/// popping on the SAME cell in CONSECUTIVE FRAMES take DISTINCT, ASCENDING stack slots, so the
/// second pop fans out ABOVE the first-frame pop that is still alive on the cell instead of
/// overlapping it.
///
/// PIN-DISCRIMINATING against the defect the retired per-frame reset counter could not
/// catch: that counter RESET every frame, so a `DotTicked` "-4" popping the frame AFTER a
/// still-alive `SuppressionApplied` "SUPPRESSED" reclaimed slot 0 and rendered at the SAME y.
/// The GTW-792 `FctSlotAllocator` counts the pops CURRENTLY ALIVE on the cell (spanning
/// frames), so the second pop takes slot 1 — this asserts the two world `y`s DIFFER.
///
/// Both updates run under a ZERO clock delta so neither pop RISES or expires: the first-frame
/// pop stays alive (and at its spawn `y`) to be counted, and any `y` difference between the two
/// pops is purely the stack-slot offset.
#[test]
fn two_families_on_one_cell_across_consecutive_frames_take_distinct_stack_slots() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));

    let cell = Cell::new(9, 9);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pinned = wounded_ganger(&mut app, cell, level, 2);

    // FRAME 1: the suppression family pops "SUPPRESSED" on the cell (slot 0 — no live pops).
    play(&mut app, SuppressionApplied::new(pinned, at));
    app.update();

    // FRAME 2: the DOT family pops "-4" on the SAME cell. The suppression pop from frame 1 is
    // still alive (zero-delta clock), so the allocator counts it and hands the DOT pop slot 1.
    play(&mut app, DotTicked::new(pinned, at, DotDamage::new(4)));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "the suppression family must pop its tag, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "-4", valence_color(FctValence::Dot)),
        "the DOT family must pop its number, got {pops:?}",
    );

    // Collect the two pops' world ys — a pop landing on a cell that already carries a live pop
    // must sit at a DISTINCT stacked height (with the retired per-frame counter both reclaimed
    // slot 0 across frames and these were EQUAL).
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &FloatingCombatText, &Transform)>();
    let mut suppressed_y: Option<f32> = None;
    let mut dot_y: Option<f32> = None;
    for (text, _, transform) in q.iter(app.world()) {
        match text.as_str() {
            "SUPPRESSED" => suppressed_y = Some(transform.translation.y),
            "-4" => dot_y = Some(transform.translation.y),
            _ => {}
        }
    }
    let (Some(suppressed_y), Some(dot_y)) = (suppressed_y, dot_y) else {
        // The has_fct_pop asserts above already failed loudly if a pop is missing.
        return;
    };
    assert!(
        (suppressed_y - dot_y).abs() > 0.001,
        "a family popping a cell that already carries a live pop from a prior frame must take a \
         DISTINCT stack slot (distinct ys), got {suppressed_y} == {dot_y} — the lifetime-aware \
         FctSlotAllocator must hand the second-frame pop slot 1",
    );
}

/// GTW-572 C4 (acceptance 4) — INERTNESS: a presenter-only app whose harness registers NO
/// consequence-family `Messages<M>` buffer updates without panicking and spawns no pops —
/// INCLUDING the field and on-death families, whose buffers the presenter used to
/// `add_message` idempotently itself (removed by C4).
///
/// The load-bearing pins: (a) after the presenter plugin built and the app updated, the
/// family buffers are STILL ABSENT — proving the registrar never calls `add_message` (the
/// old renderer walls did, for `FieldTicked` / `OnDeathOccurred`); (b) the gated family
/// readers stay inert (no `FloatingCombatText` spawns, no param-validation panic) across
/// several updates of a live-battle app.
#[test]
fn a_presenter_only_app_with_no_family_buffers_stays_inert() {
    // Deliberately NOT the shared harness: no add_message for ANY consequence family
    // (no Bleeding / ArmorBroken / SuppressionApplied / DotTicked / InjuryInflicted /
    // FieldTicked / OnDeathOccurred).
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
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A live battle, FxTuning resolved, several updates — the family readers must all stay
    // gated off their absent buffers (no panic, nothing spawned).
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "with no family Messages<M> buffer registered, no consequence pop may spawn",
    );
    // The C4 pin: the registrar must NOT have add_message'd any family buffer — including
    // the two the old renderer walls registered idempotently (FieldTicked, OnDeathOccurred).
    assert!(
        app.world()
            .get_resource::<Messages<gdtf_battle_sim::effects::fields::FieldTicked>>()
            .is_none(),
        "the presenter must no longer register Messages<FieldTicked> itself (GTW-572 C4)",
    );
    assert!(
        app.world()
            .get_resource::<Messages<OnDeathOccurred>>()
            .is_none(),
        "the presenter must no longer register Messages<OnDeathOccurred> itself (GTW-572 C4)",
    );
    for absent in [
        app.world()
            .get_resource::<Messages<Bleeding>>()
            .map(|_| "Bleeding"),
        app.world()
            .get_resource::<Messages<ArmorBroken>>()
            .map(|_| "ArmorBroken"),
        app.world()
            .get_resource::<Messages<SuppressionApplied>>()
            .map(|_| "SuppressionApplied"),
        app.world()
            .get_resource::<Messages<DotTicked>>()
            .map(|_| "DotTicked"),
        app.world()
            .get_resource::<Messages<InjuryInflicted>>()
            .map(|_| "InjuryInflicted"),
        // GTW-623 C4: the four buffers the renderer plugin used to `add_message`
        // idempotently itself (the vacuous registrations that made its own
        // `resource_exists::<Messages<M>>` gates always-true). The sim's plugins register
        // them in a live battle; a presenter-only app must NOT carry them.
        app.world()
            .get_resource::<Messages<MeleeResolved>>()
            .map(|_| "MeleeResolved"),
        app.world()
            .get_resource::<Messages<FallOccurred>>()
            .map(|_| "FallOccurred"),
        app.world()
            .get_resource::<Messages<ThrowResolved>>()
            .map(|_| "ThrowResolved"),
        app.world()
            .get_resource::<Messages<SlabDestroyed>>()
            .map(|_| "SlabDestroyed"),
    ] {
        assert!(
            absent.is_none(),
            "the presenter registrar must not register the {absent:?} family buffer",
        );
    }
}

/// GTW-623 C4 / A2 — the ONE allowed sim-owned `add_message` exception, pinned: the
/// presenter registers `Messages<ShotFired>` itself (in `plugin/topdown/gangers.rs`, the
/// documented exception) so `update_ganger_life_state`'s `MessageReader<ShotFired>` stays
/// valid — and the ganger batch keeps running — in a fire-less presenter-only harness (the
/// `ganger_draw` / `fog_present` suites register no sim buffer at all).
///
/// If a refactor either DROPS the exception (the buffer goes absent — those harnesses would
/// panic param validation) or ADDS more sim-owned registrations (caught by the absent-buffer
/// pins above), this contract goes red.
#[test]
fn the_presenter_registers_exactly_the_one_documented_sim_buffer_exception() {
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
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    app.update();

    assert!(
        app.world().get_resource::<Messages<ShotFired>>().is_some(),
        "the presenter must register Messages<ShotFired> itself — THE one documented \
         sim-owned add_message exception (GTW-623 C4): update_ganger_life_state must keep \
         running in fire-less harnesses",
    );
}

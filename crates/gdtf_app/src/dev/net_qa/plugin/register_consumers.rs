//! The per-request consumer registrations and their ordering (GTW-737 … GTW-802).
//!
//! Every consumer runs `.after(route_requests)` so it drains the request the router just
//! routed the SAME frame; the per-consumer comments record the extra ordering and the
//! run-condition each one needs.

use bevy::prelude::*;
use gdtf_battle_input::{ContextualActSystems, InputSystems, dispatch_act_intents};
use gdtf_battle_sim::{occupancy_sync::SimSystems, prelude::BattleInProgress};
use gdtf_ui::focus_nav::FocusNavSystems;

use crate::dev::net_qa::{
    activate_menu::drive_activate_menu_item,
    events::{drive_output, reset_output_cursor_when_idle},
    focus_control::drive_focus_control,
    inject::apply_injects,
    router::route_requests,
    screenshot::drive_screenshots,
    screenshot_after::{claim_screenshot_after, tick_after_shots},
    snapshot::build_snapshots,
    start_battle::drive_start_battle,
    stepper::drive_stepper_control,
};

/// Register every per-request consumer system (see the per-block comments for each one's
/// ordering and run condition).
pub(super) fn register_consumers(app: &mut App) {
    register_battle_consumers(app);
    register_screenshot_consumers(app);
    register_navigation_consumers(app);
}

/// The battle-gated consumers: the T4 inject pump, the T5 snapshot service, and the T6
/// outbox pump (plus its always-on cursor reset).
fn register_battle_consumers(app: &mut App) {
    // GTW-737 — the T4 inject pump. In the SAME `InputSystems::Gather` band, ordered
    // `.after(route_requests)` (so it drains the request the router just routed) and
    // `.before` BOTH intent drains ([`ContextualActSystems::Drain`] and
    // [`dispatch_act_intents`]) — so an intent it pushes is drained, and its `*Requested`
    // sim-consumed, the SAME frame (the co-schedule same-frame guarantee). Gated on a live
    // battle (the only state its input-queue resources exist in; the router already rejects
    // off-battle injects `NoBattle`), so it never runs — nor validates its params — when a
    // battle is absent (`bevy-traps.md` #1).
    app.add_systems(
        Update,
        apply_injects
            .in_set(InputSystems::Gather)
            .after(route_requests)
            .before(ContextualActSystems::Drain)
            .before(dispatch_act_intents)
            .run_if(resource_exists::<BattleInProgress>),
    );
    // GTW-738 — the T5 snapshot service. Registered `.after(SimSystems::Simulate)` (the
    // post-Simulate observation point), so it reads the LIVE post-Simulate world: a
    // `GetBattleState` routed this frame (the router runs in `InputSystems::Gather`, ordered
    // before `SimSystems::Simulate`) is drained and answered the SAME frame, reflecting that
    // frame's mutations rather than a stale cache. Gated on a live battle (its battle-lifetime
    // resources exist only then, and the router already rejects off-battle `GetBattleState`
    // `NoBattle`), so it never runs — nor validates its params — off-battle (`bevy-traps.md` #1).
    app.add_systems(
        Update,
        build_snapshots
            .after(SimSystems::Simulate)
            .run_if(resource_exists::<BattleInProgress>),
    );
    // GTW-739 — the T6 outbox pump. Registered `.after(SimSystems::Record)` (the act-log
    // write point) so a `GetOutput` routed this frame is drained and answered the SAME
    // frame, reflecting every act the sim recorded up to now — the router runs in
    // `InputSystems::Gather`, ordered before `SimSystems::Simulate`, and `Record` runs
    // after `Simulate`, so the ordering within one update is route → Simulate → Record →
    // drive_output. Gated on a live battle (the `ActLog` it projects is battle-lifetime,
    // inserted/removed with `BattleInProgress`, and the router only enqueues `GetOutput`
    // in a battle), so it never runs — nor validates its params — off-battle
    // (`bevy-traps.md` #1). Unlike the inject pump it is NOT gated on the presenter
    // catching up (GTW-727 C44): `GetOutput` is the client's observation channel.
    app.add_systems(
        Update,
        drive_output
            .after(SimSystems::Record)
            .run_if(resource_exists::<BattleInProgress>),
    );
    // GTW-739 — the T6 cursor's between-battles reset. Runs ALWAYS (not gated on a battle)
    // so it observes the absent act log between battles and rewinds the cursor to the
    // battle start, exactly as the presenter's playback cursor self-heals on an absent
    // `ActLog`. Ordered before `drive_output` so a reset lands before the same frame's
    // drain; it takes `Option<Res<ActLog>>` and never panics off-battle.
    app.add_systems(Update, reset_output_cursor_when_idle.before(drive_output));
}

/// The capture consumers: the T7 screenshot pump and the T15 frame-exact deferred capture.
fn register_screenshot_consumers(app: &mut App) {
    // GTW-740 — the T7 screenshot pump. In the `InputSystems::Gather` band ordered
    // `.after(route_requests)` so it claims the `TakeScreenshot` the router just routed the
    // SAME frame, then holds it across frames while the GPU readback flushes the PNG (the
    // reply is genuinely deferred — see `drive_screenshots`). Runs unconditionally: a
    // screenshot needs no live battle, and `TakeScreenshot` is not battle-dependent at the
    // router.
    app.add_systems(
        Update,
        drive_screenshots
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
    // GTW-749 — the T15 screenshot-after child. `tick_after_shots` runs UNCONDITIONALLY
    // (a capture already queued must keep counting down and fire even past a battle
    // ending) `.after(route_requests)`; `claim_screenshot_after` runs `.after` it (so a
    // freshly-claimed entry is never ticked the frame it is created — the T7
    // poll-before-claim discipline applied here too) and shares `apply_injects`'
    // ordering + battle gate: `.before(ContextualActSystems::Drain)` +
    // `.before(dispatch_act_intents)` (the T4 same-frame co-schedule guarantee for the
    // intent it injects) and `run_if(resource_exists::<BattleInProgress>)` (the router
    // already rejects an off-battle `ScreenshotAfter` `NoBattle` before it is ever
    // queued, exactly like a bare `Inject`).
    app.add_systems(
        Update,
        tick_after_shots
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
    app.add_systems(
        Update,
        claim_screenshot_after
            .in_set(InputSystems::Gather)
            .after(route_requests)
            .after(tick_after_shots)
            .before(ContextualActSystems::Drain)
            .before(dispatch_act_intents)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// The always-on navigation / UI-drive consumers: start-battle, the DEV stepper drive, the
/// menu-item activation, and the generic focus drive.
fn register_navigation_consumers(app: &mut App) {
    // GTW-742 — the T9 start-battle navigation consumer. In the `InputSystems::Gather`
    // band ordered `.after(route_requests)` so it drains the `StartBattle` the router just
    // routed the SAME frame. It produces the SAME `StartBattleRequested` the menu's
    // Battlescape button produces (the menu's `apply_start_battle` performs the actual
    // transition + seed install), so both the network path and the local UI path share one
    // truth. Runs unconditionally: it must be able to reject a `StartBattle` that arrives in
    // the wrong state (not at the menu, or mid-battle) with a typed error rather than leave
    // it to the deadline sweep.
    app.add_systems(
        Update,
        drive_start_battle
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
    // GTW-766 — the DEV procgen stepper-drive dispatch. In the `InputSystems::Gather` band
    // ordered `.after(route_requests)` so it drains the `StepperControl` the router just
    // routed the SAME frame, writing it into the SAME latch the egui panel's Next/Auto/Skip
    // buttons write. Runs UNCONDITIONALLY (both its `dev_tools` and stub bodies): the router
    // already route-rejects a `StepperControl` with no live drive (`StepperInactive`), so this
    // only ever drains a command the router accepted — but it must run even without
    // `dev_tools` to drain-and-answer the queue rather than leaving it to the deadline sweep.
    app.add_systems(
        Update,
        drive_stepper_control
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
    // GTW-787 — the menu-item activation consumer. In the `InputSystems::Gather` band
    // ordered `.after(route_requests)` so it drains the `ActivateMenuItem` the router just
    // routed the SAME frame. It raises the SAME `FocusActivated` message a keyboard `Enter`
    // raises for the token'd menu item — the real activation path — so a scene's own
    // activation consumer reacts identically. Runs UNCONDITIONALLY (menus exist off-battle):
    // it must be able to answer a stale token with a typed rejection rather than leave it to
    // the deadline sweep.
    app.add_systems(
        Update,
        drive_activate_menu_item
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
    // GTW-802 — the generic focus-drive consumer. In the `InputSystems::Gather` band
    // ordered `.after(route_requests)` so it drains the `FocusControl` the router just
    // routed the SAME frame, and `.before(FocusNavSystems::Apply)` — the `gdtf_ui` set that
    // drains `NavigateRequest` — so a `Step` raised this frame moves focus this frame rather
    // than dropping one (`bevy-traps.md` #3). Runs UNCONDITIONALLY and is never battle-gated:
    // the focus-navigable screens it drives (Options, the menu) are off-battle, which is
    // exactly the gap it closes, and it must be able to answer a stale token with a typed
    // rejection rather than leave it to the deadline sweep.
    app.add_systems(
        Update,
        drive_focus_control
            .in_set(InputSystems::Gather)
            .after(route_requests)
            .before(FocusNavSystems::Apply),
    );
}

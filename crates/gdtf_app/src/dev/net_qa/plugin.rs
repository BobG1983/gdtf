//! [`NetQaPlugin`] — the DEV-ONLY QA network control channel's registration
//! (GTW-736).
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **`cfg(all(debug_assertions, feature = "net_qa"))`.** The wiring site
//!    ([`crate::dev::plugin`]) only adds the plugin under a debug build AND the opt-in
//!    `net_qa` feature — it opens a listener, so a release artifact never sees it, even
//!    with the feature on.
//! 2. **Opt-in env var.** Even in a `net_qa` debug build the plugin is INERT by default:
//!    [`from_env`](NetQaPlugin::from_env) reads `GDTF_NET_QA` and registers NOTHING unless
//!    it is set truthy, so a normal `cargo run` reaches a battle exactly as before.
//!
//! ## Wiring when active
//!
//! - Binds the loopback listener (`Ipv4Addr::LOCALHOST` + `GDTF_NET_QA_PORT`) on a
//!   `std::thread` and inserts the [`NetInbox`] the listener pushes decoded requests to.
//!   That accept-loop thread is a DELIBERATE process-lifetime daemon: `build` discards its
//!   `JoinHandle`, and it runs (with no in-app shutdown / `AppExit` teardown) until the
//!   process exits and the OS reaps it — a dev-only QA channel's lifetime IS the app's. The
//!   full rationale is on [`run_listener`](super::listener::run_listener).
//! - Inits the five typed [`PendingQueue`]s and registers the deadline
//!   [`sweep_pending`] pumps + the always-on [`route_requests`] router, all in the
//!   [`InputSystems::Gather`] band (sweeps chained before the router so a freshly-enqueued
//!   request gets its full deadline budget). The screenshot queue is the ONE exception — it
//!   has no sweep; the T7 [`drive_screenshots`] pump owns its lifetime end to end (it claims
//!   every screenshot the frame it is routed and runs its own multi-frame poll timeout).

use std::{sync::mpsc, thread};

use bevy::prelude::*;
use gdtf_battle_input::{ContextualActSystems, InputSystems, dispatch_act_intents};
use gdtf_battle_sim::{occupancy_sync::SimSystems, prelude::BattleInProgress};

use super::{
    channel::{IncomingRequest, NetInbox},
    config::{DEFAULT_IO_TIMEOUT, NetIoTimeout, NetQaPort},
    env::{net_qa_enabled, port_from_env},
    events::{QaOutputCursor, drive_output, reset_output_cursor_when_idle},
    inject::apply_injects,
    listener::{bind_listener, run_listener},
    pending::{
        InjectPayload, OutputPayload, PendingQueue, ScreenshotAfterPayload, ScreenshotPayload,
        SnapshotPayload, StartBattlePayload, sweep_pending,
    },
    router::route_requests,
    screenshot::{InFlightShots, QaShotDir, ShotPollBudget, ShotSequence, drive_screenshots},
    screenshot_after::{AfterShotQueue, claim_screenshot_after, tick_after_shots},
    snapshot::build_snapshots,
    start_battle::drive_start_battle,
};

/// How a [`NetQaPlugin`] instance activates on `build`.
enum Wiring {
    /// Inert (the env gate is off) — `build` registers nothing.
    Disabled,
    /// Env-driven: bind + spawn the loopback listener on `build`.
    Listener {
        /// The loopback port to bind.
        port:       NetQaPort,
        /// The two-sided socket timeout.
        io_timeout: NetIoTimeout,
    },
    /// Test-driven: no listener; the inbox receiver is injected for a headless routing
    /// test. `Mutex<Option<…>>` so `build` can `take` the non-`Clone` receiver out of
    /// `&self`.
    #[cfg(feature = "test-support")]
    Channels {
        /// The injected request receiver, taken on `build`.
        inbox: std::sync::Mutex<Option<mpsc::Receiver<IncomingRequest>>>,
    },
}

crate::support_item! {
    /// The DEV-ONLY QA network control channel plugin.
    struct NetQaPlugin {
        /// How this instance activates on `build`.
        wiring: Wiring,
    }
}

impl NetQaPlugin {
    crate::support_item! {
        /// Construct the plugin, reading its `GDTF_NET_QA` / `GDTF_NET_QA_PORT` env gates.
        ///
        /// When `GDTF_NET_QA` is truthy the plugin binds the loopback listener on
        /// `build`; otherwise it is inert (registers nothing), so a normal launch is
        /// unaffected. This is the constructor `crate::dev::plugin` uses under
        /// `cfg(all(debug_assertions, feature = "net_qa"))`.
        #[must_use]
        fn from_env() -> Self {
            let wiring = if net_qa_enabled() {
                Wiring::Listener {
                    port:       port_from_env(),
                    io_timeout: DEFAULT_IO_TIMEOUT,
                }
            } else {
                Wiring::Disabled
            };
            Self { wiring }
        }
    }

    /// Construct the plugin wired to an injected inbox receiver, with NO listener thread —
    /// the headless routing-test constructor.
    ///
    /// The test keeps the paired sender and pushes [`IncomingRequest`]s exactly as the
    /// listener would, then drives `app.update()` and reads the router's replies.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn with_channels(inbox: mpsc::Receiver<IncomingRequest>) -> Self {
        Self {
            wiring: Wiring::Channels {
                inbox: std::sync::Mutex::new(Some(inbox)),
            },
        }
    }

    /// Bind the REAL loopback listener on an OS-assigned ephemeral port and spawn its
    /// accept loop, returning the bound port and the request receiver — the transport
    /// test entry point.
    ///
    /// The test connects to the returned port and drives a stand-in for the Bevy side off
    /// the returned receiver (the router itself is covered by
    /// [`with_channels`](Self::with_channels)). Binding port `0` yields a fresh port per
    /// call, so parallel tests never collide.
    ///
    /// # Errors
    ///
    /// Any [`std::io::Error`] from binding the loopback listener.
    #[cfg(feature = "test-support")]
    pub fn spawn_test_listener(
        io_timeout: NetIoTimeout,
    ) -> std::io::Result<(NetQaPort, mpsc::Receiver<IncomingRequest>)> {
        let (tx, rx) = mpsc::channel::<IncomingRequest>();
        let (listener, port) = bind_listener(NetQaPort::new(0))?;
        // Detached accept-loop daemon, exactly as the live arm — the discarded
        // `JoinHandle` is deliberate: the thread runs until the test process exits, which
        // reaps it (see `run_listener`'s "Thread lifetime & shutdown" note).
        thread::spawn(move || run_listener(listener, tx, io_timeout));
        Ok((port, rx))
    }
}

impl Default for NetQaPlugin {
    /// The wiring default: read the env-var gate.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for NetQaPlugin {
    fn build(&self, app: &mut App) {
        match &self.wiring {
            Wiring::Disabled => {}
            Wiring::Listener { port, io_timeout } => {
                let (tx, rx) = mpsc::channel::<IncomingRequest>();
                let Ok((listener, bound)) = bind_listener(*port) else {
                    error!(
                        port = **port,
                        "net_qa: failed to bind the loopback listener — QA channel OFF"
                    );
                    return;
                };
                info!(
                    port = *bound,
                    "net_qa: ON (dev) — loopback QA control channel listening"
                );
                let io_timeout = *io_timeout;
                // Detached accept-loop daemon — the discarded `JoinHandle` is deliberate:
                // this dev-only channel's lifetime IS the app's, so the thread runs for the
                // process lifetime and the OS reaps it (and closes the socket) on exit. See
                // `run_listener`'s "Thread lifetime & shutdown" note for the full rationale.
                thread::spawn(move || run_listener(listener, tx, io_timeout));
                app.insert_resource(NetInbox::new(rx));
                register_router(app);
            }
            #[cfg(feature = "test-support")]
            Wiring::Channels { inbox } => {
                let Some(rx) = inbox.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                app.insert_resource(NetInbox::new(rx));
                register_router(app);
            }
        }
    }
}

/// Init the five typed pending queues and register the sweeps + router + the T4 inject
/// pump in the [`InputSystems::Gather`] band (sweeps chained before the router).
fn register_router(app: &mut App) {
    app.init_resource::<PendingQueue<InjectPayload>>()
        .init_resource::<PendingQueue<SnapshotPayload>>()
        .init_resource::<PendingQueue<OutputPayload>>()
        .init_resource::<PendingQueue<ScreenshotPayload>>()
        .init_resource::<PendingQueue<ScreenshotAfterPayload>>()
        .init_resource::<PendingQueue<StartBattlePayload>>()
        // GTW-740 — the T7 screenshot pump's own state: the captures in-flight across
        // frames, the poll budget, the confinement directory, and the monotonic sequence
        // that makes every capture's output path unique.
        .init_resource::<InFlightShots>()
        .init_resource::<ShotPollBudget>()
        .init_resource::<QaShotDir>()
        .init_resource::<ShotSequence>()
        // GTW-749 — the T15 screenshot-after child's own state: the accepted requests
        // counting down to their fire frame.
        .init_resource::<AfterShotQueue>()
        // GTW-739 — the T6 outbox's own read position over the act log. Process-lifetime
        // (it self-heals across battles), so it is init'd here beside the other pump state
        // rather than at a battle boundary.
        .init_resource::<QaOutputCursor>();
    // NOTE: neither the `ScreenshotPayload` nor the `ScreenshotAfterPayload` queue has a
    // deadline sweep — the T7 pump (`drive_screenshots`, below) claims every screenshot
    // the frame it is routed and owns its own multi-frame poll timeout, and the T15
    // `claim_screenshot_after` claims every screenshot-after the frame it is routed too
    // (answering `Rejected` immediately or handing it to `AfterShotQueue`'s OWN countdown)
    // — a generic sweep would only ever answer the wrong reply type for either. Every
    // OTHER queue keeps its sweep.
    app.add_systems(
        Update,
        (
            sweep_pending::<InjectPayload>,
            sweep_pending::<SnapshotPayload>,
            sweep_pending::<OutputPayload>,
            sweep_pending::<StartBattlePayload>,
            route_requests,
        )
            .chain()
            .in_set(InputSystems::Gather),
    );
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
}

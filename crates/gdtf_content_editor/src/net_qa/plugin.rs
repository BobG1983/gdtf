//! [`NetQaEditorPlugin`] — the editor's DEV-ONLY QA network control channel registration
//! (GTW-804).
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **`cfg(all(debug_assertions, feature = "net_qa"))`.** The wiring site (`crate::app`)
//!    only adds the plugin under a debug build AND the opt-in `net_qa` feature — it opens a
//!    listener, so a release editor never sees it, even with the feature on.
//! 2. **Opt-in env var.** Even in a `net_qa` debug editor the plugin is INERT by default:
//!    [`NetQaEditorPlugin::from_env`] reads `GDTF_EDITOR_NET_QA` and registers NOTHING unless
//!    it is set truthy, so a normal editor launch is unaffected.
//!
//! Loopback-only binding and the one-client-at-a-time gate are the shared transport's
//! (`gdtf_net_qa_transport`, GTW-803) and are not restated here — the editor gets exactly the
//! game's enforcement because it runs the game's code.
//!
//! ## Wiring when active
//!
//! - Binds the loopback listener (`Ipv4Addr::LOCALHOST` + `GDTF_EDITOR_NET_QA_PORT`) on a
//!   `std::thread` and inserts the [`NetInbox`] the listener pushes decoded requests to. That
//!   accept-loop thread is a DELIBERATE process-lifetime daemon: `build` discards its
//!   `JoinHandle` and it runs until the process exits, exactly as the game's does — the full
//!   rationale is on [`run_listener`](gdtf_net_qa_transport::run_listener).
//! - Registers the request drain in the [`EditorNetQaSystems::Gather`] band.

use std::{
    io,
    net::TcpListener,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use gdtf_net_qa_transport::{
    DEFAULT_IO_TIMEOUT, IncomingRequest, NetInbox, NetIoTimeout, NetQaPort, PendingQueue,
    bind_listener, run_listener, sweep_pending,
};

use super::{
    env::{editor_net_qa_enabled, editor_port_from_env},
    present::EditorCapturePresentPlugin,
    router::route_editor_requests,
    schedule::EditorNetQaSystems,
    screenshot::{
        EditorInFlightShots, EditorQaShotDir, EditorScreenshotPayload, EditorShotPollBudget,
        EditorShotSequence, EditorShotSettle, EditorShotSource, drive_editor_screenshots,
    },
};
use crate::EditorState;

/// How a [`NetQaEditorPlugin`] instance activates on `build`.
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
    /// Pre-bound: the listener was bound by [`NetQaEditorPlugin::listening`] before the app
    /// was built, so the caller already knows the port. `Mutex<Option<…>>` so `build` can
    /// `take` the non-`Clone` [`TcpListener`] out of `&self`.
    Bound {
        /// The already-bound loopback listener, taken on `build`.
        listener:   Mutex<Option<TcpListener>>,
        /// The two-sided socket timeout.
        io_timeout: NetIoTimeout,
    },
}

/// The editor's DEV-ONLY QA network control channel plugin.
pub struct NetQaEditorPlugin {
    /// How this instance activates on `build`.
    wiring: Wiring,
}

impl NetQaEditorPlugin {
    /// Construct the plugin, reading its `GDTF_EDITOR_NET_QA` / `GDTF_EDITOR_NET_QA_PORT` env
    /// gates.
    ///
    /// When `GDTF_EDITOR_NET_QA` is truthy the plugin binds the loopback listener on `build`;
    /// otherwise it is inert (registers nothing), so a normal launch is unaffected. This is
    /// the constructor the editor's app wiring (`crate::app`) uses under
    /// `cfg(all(debug_assertions, feature = "net_qa"))` — the ONLY one the editor binary ever
    /// reaches.
    #[must_use]
    pub fn from_env() -> Self {
        let wiring = if editor_net_qa_enabled() {
            Wiring::Listener {
                port:       editor_port_from_env(),
                io_timeout: DEFAULT_IO_TIMEOUT,
            }
        } else {
            Wiring::Disabled
        };
        Self { wiring }
    }

    /// Construct the plugin around a listener bound RIGHT NOW on `port`, returning it with
    /// the port actually bound — the integration test's constructor.
    ///
    /// Binding eagerly is what makes the test deterministic: it passes port `0`, the OS picks
    /// a free port (so parallel test binaries never collide), and the caller learns that port
    /// immediately rather than racing the app's first `build`. It exists because the env gate
    /// itself cannot be driven from a test — `std::env::set_var` is `unsafe` in edition 2024
    /// and the workspace forbids `unsafe` — so the alternative would be a test that never
    /// touches the real listener at all. It does NOT weaken the gate: like everything in this
    /// module it compiles only under `cfg(all(debug_assertions, feature = "net_qa"))`, and the
    /// binary always goes through [`from_env`](Self::from_env).
    ///
    /// # Errors
    ///
    /// Any [`io::Error`] from binding the loopback listener (e.g. `port` already in use).
    pub fn listening(port: NetQaPort) -> io::Result<(Self, NetQaPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener:   Mutex::new(Some(listener)),
                io_timeout: DEFAULT_IO_TIMEOUT,
            },
        };
        Ok((plugin, bound))
    }
}

impl Default for NetQaEditorPlugin {
    /// The wiring default: read the env-var gate.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for NetQaEditorPlugin {
    fn build(&self, app: &mut App) {
        match &self.wiring {
            Wiring::Disabled => {}
            Wiring::Listener { port, io_timeout } => {
                let Ok((listener, bound)) = bind_listener(*port) else {
                    error!(
                        port = **port,
                        "editor net_qa: failed to bind the loopback listener — QA channel OFF"
                    );
                    return;
                };
                info!(
                    port = *bound,
                    "editor net_qa: ON (dev) — loopback QA control channel listening"
                );
                serve(app, listener, *io_timeout);
            }
            Wiring::Bound {
                listener,
                io_timeout,
            } => {
                let Some(listener) = listener.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                serve(app, listener, *io_timeout);
            }
        }
    }
}

/// Spawn the accept loop over an already-bound `listener` and register the host side: the
/// inbox it pushes into, and the request drain that answers from it.
///
/// The drain runs in [`Update`], in the [`EditorNetQaSystems::Gather`] band (see that set for
/// why it is not in the egui pass), under the EDITOR's own state machine: the run condition is
/// the existence of [`State<EditorState>`] — never the game's `AppState`, which this crate
/// does not have and must not depend on.
///
/// It is deliberately NOT narrowed to [`EditorState::Editing`]. Version negotiation must be
/// answerable in EVERY editor state: a client that connects during the `Load` asset pass and
/// waits for a reply would otherwise be reaped by the socket's own read timeout before the
/// editor ever reaches `Editing`.
///
/// Takes `&mut App` (the ordinary app-builder handle, as [`App::add_plugins`] does); not a
/// registered system nor a `&mut World` helper, so bevy-traps #7 does not apply.
fn serve(app: &mut App, listener: TcpListener, io_timeout: NetIoTimeout) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    // Detached accept-loop daemon — the discarded `JoinHandle` is deliberate: this dev-only
    // channel's lifetime IS the editor's, so the thread runs for the process lifetime and the
    // OS reaps it (and closes the socket) on exit. See `run_listener`'s "Thread lifetime &
    // shutdown" note for the full rationale.
    thread::spawn(move || run_listener(listener, tx, io_timeout));
    app.insert_resource(NetInbox::new(rx));
    // The screenshot pump's queue, tracking set, uniqueness counter and three tunables
    // (GTW-880). `init_resource` leaves a value a caller already inserted alone, so a test
    // that pins a temp directory / a short settle keeps it.
    app.init_resource::<PendingQueue<EditorScreenshotPayload>>();
    app.init_resource::<EditorInFlightShots>();
    app.init_resource::<EditorShotSequence>();
    app.init_resource::<EditorQaShotDir>();
    app.init_resource::<EditorShotSettle>();
    app.init_resource::<EditorShotPollBudget>();
    app.init_resource::<EditorShotSource>();
    // GTW-918: the offscreen capture-target present path. Added HERE, on the listener arm only,
    // so an inert plugin (and every non-`net_qa` / release editor) keeps rendering straight to
    // the window with no offscreen target, no present camera and no `WinitSettings` override.
    // It replaces the `init_resource` default's placeholder handle with the real target once the
    // editor has a sized primary window.
    app.add_plugins(EditorCapturePresentPlugin);
    app.configure_sets(
        Update,
        EditorNetQaSystems::Gather.run_if(resource_exists::<State<EditorState>>),
    );
    // Chained, in this order: the drain routes a `TakeScreenshot` onto the pending queue, the
    // pump claims it the SAME frame (so it never reaches the sweep), and the shared
    // transport's `sweep_pending` answers a typed `Timeout` on anything that somehow went
    // unclaimed rather than leaving the client hanging.
    app.add_systems(
        Update,
        (
            route_editor_requests,
            drive_editor_screenshots,
            sweep_pending::<EditorScreenshotPayload>,
        )
            .chain()
            .in_set(EditorNetQaSystems::Gather),
    );
}

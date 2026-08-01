//! The [`NetQaPlugin`] type, its constructors, and its `Plugin` impl (GTW-736).
//!
//! ## Wiring when active
//!
//! - Binds the loopback listener (`Ipv4Addr::LOCALHOST` + `GDTF_NET_QA_PORT`) on a
//!   `std::thread` and inserts the [`NetInbox`] the listener pushes decoded requests to.
//!   That accept-loop thread is a DELIBERATE process-lifetime daemon: `build` discards its
//!   `JoinHandle`, and it runs (with no in-app shutdown / `AppExit` teardown) until the
//!   process exits and the OS reaps it — a dev-only QA channel's lifetime IS the app's. The
//!   full rationale is on [`run_listener`](gdtf_net_qa_transport::run_listener).
//! - Registers the transport half ([`register_transport`]: the capture queue, the pump's
//!   cross-frame state, the always-on router and the game's command set) and then the one
//!   consumer that hangs off it, the capture pump ([`register_consumers`]).

use std::{net::TcpListener, sync::mpsc, thread};

use bevy::prelude::*;
use gdtf_net_qa_transport::{
    DEFAULT_IO_TIMEOUT, IncomingRequest, NetInbox, NetIoTimeout, NetQaPort, bind_listener,
    run_listener,
};

use super::{register_consumers::register_consumers, register_transport::register_transport};
use crate::dev::net_qa::{
    config::hello_facts,
    env::{net_qa_enabled, port_from_env},
    present::CapturePresentPlugin,
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
    /// Pre-bound: the listener was bound by [`NetQaPlugin::listening`] before the app was
    /// built, so the caller already knows the port. `Mutex<Option<…>>` so `build` can `take`
    /// the non-`Clone` [`TcpListener`] out of `&self`.
    #[cfg(feature = "test-support")]
    Bound {
        /// The already-bound loopback listener, taken on `build`.
        listener:   std::sync::Mutex<Option<TcpListener>>,
        /// The two-sided socket timeout.
        io_timeout: NetIoTimeout,
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

    /// Construct the plugin around a listener bound RIGHT NOW on `port`, returning it with
    /// the port actually bound — the socket integration test's constructor, matching the
    /// editor's `NetQaEditorPlugin::listening`.
    ///
    /// Everything downstream of the bind is the production path: `build` runs the SAME
    /// `serve` the env-driven [`from_env`](Self::from_env) arm runs, so the handshake facts
    /// the listener thread answers with are the game's own, read from the same call site.
    ///
    /// Binding eagerly is what makes the test deterministic: it passes port `0`, the OS picks
    /// a free port (so parallel test binaries never collide), and the caller learns that port
    /// immediately rather than racing the app's first `build`. It exists because the env gate
    /// the binary uses cannot be driven from a test — `std::env::set_var` is `unsafe` in
    /// edition 2024 and the workspace forbids `unsafe` — so the alternative would be a test
    /// that never opens the real listener at all.
    ///
    /// # Errors
    ///
    /// Any [`std::io::Error`] from binding the loopback listener (e.g. `port` already in use).
    #[cfg(feature = "test-support")]
    pub fn listening(port: NetQaPort) -> std::io::Result<(Self, NetQaPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener:   std::sync::Mutex::new(Some(listener)),
                io_timeout: DEFAULT_IO_TIMEOUT,
            },
        };
        Ok((plugin, bound))
    }
}

/// Spawn the accept loop over an already-bound `listener` and register the host side: the
/// inbox it pushes into, the transport half, every consumer, and the offscreen-capture
/// present path.
///
/// The ONE place the game states its handshake facts to the transport. Both listener arms
/// (`from_env`'s bind and `listening`'s pre-bound one) go through here, so the facts a real
/// client negotiates against are the facts [`hello_facts`] returns and nothing else.
///
/// Takes `&mut App` (the ordinary app-builder handle, as [`App::add_plugins`] does); not a
/// registered system nor a `&mut World` helper, so bevy-traps #7 does not apply.
fn serve(app: &mut App, listener: TcpListener, io_timeout: NetIoTimeout) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    // Detached accept-loop daemon — the discarded `JoinHandle` is deliberate:
    // this dev-only channel's lifetime IS the app's, so the thread runs for the
    // process lifetime and the OS reaps it (and closes the socket) on exit. See
    // `run_listener`'s "Thread lifetime & shutdown" note for the full rationale.
    // This host's OWN handshake facts: the listener thread negotiates every
    // `Hello` from them (GTW-940), so the game answers with `gdtf-net-qa` and the
    // editor with its own name from one implementation.
    thread::spawn(move || run_listener(listener, tx, io_timeout, hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    register_transport(app);
    register_consumers(app);
    // GTW-764: retarget the game cameras to an offscreen image + blit it back to
    // the window, so a QA capture reads pixels the render graph writes every tick
    // — a backgrounded macOS window's swapchain reads back BLACK. Only the
    // listener arms get it (the router-only test arm and a non-`net_qa` / release
    // build render the normal window swapchain path).
    app.add_plugins(CapturePresentPlugin);
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
                serve(app, listener, *io_timeout);
            }
            #[cfg(feature = "test-support")]
            Wiring::Bound {
                listener,
                io_timeout,
            } => {
                let Some(listener) = listener.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                serve(app, listener, *io_timeout);
            }
            #[cfg(feature = "test-support")]
            Wiring::Channels { inbox } => {
                let Some(rx) = inbox.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                app.insert_resource(NetInbox::new(rx));
                register_transport(app);
                register_consumers(app);
            }
        }
    }
}

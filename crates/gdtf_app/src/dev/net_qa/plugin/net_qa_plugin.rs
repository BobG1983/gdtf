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
//! - Registers the transport half ([`register_transport`]: the typed pending queues, the
//!   pump state, the deadline sweeps + the always-on router) and then every per-request
//!   consumer ([`register_consumers`]).

use std::{sync::mpsc, thread};

use bevy::prelude::*;
use gdtf_net_qa_transport::{
    DEFAULT_IO_TIMEOUT, IncomingRequest, NetInbox, NetIoTimeout, NetQaPort, bind_listener,
    run_listener,
};

use super::{register_consumers::register_consumers, register_transport::register_transport};
use crate::dev::net_qa::{
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
                register_transport(app);
                register_consumers(app);
                // GTW-764: retarget the game cameras to an offscreen image + blit it back to
                // the window, so `TakeScreenshot` / `ScreenshotAfter` capture pixels the render
                // graph writes every tick — a backgrounded macOS window's swapchain reads back
                // BLACK. Only the env-active listener arm gets it (the router-only test arm and
                // a non-`net_qa` / release build render the normal window swapchain path).
                app.add_plugins(CapturePresentPlugin);
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

//! MCP plugin wiring for the game binary.

use std::{net::TcpListener, sync::mpsc, thread};

use bevy::prelude::*;
use cobalt_mcp_host::{IncomingRequest, NetInbox, bind_listener, run_listener};
use cobalt_mcp_protocol::{ports::McpPort, timeouts::NetTimeouts};

use super::{
    register_consumers::register_consumers, register_present::register_present,
    register_transport::register_transport,
};
use crate::dev::mcp::{config::hello_facts, env::port_from_env};

enum Wiring {
    Disabled,
    Listener {
        port:     McpPort,
        timeouts: NetTimeouts,
    },
    #[cfg(feature = "headless_test")]
    Channels {
        inbox: std::sync::Mutex<Option<mpsc::Receiver<IncomingRequest>>>,
    },
    #[cfg(feature = "headless_test")]
    Bound {
        listener: std::sync::Mutex<Option<TcpListener>>,
        timeouts: NetTimeouts,
    },
}

crate::support_item! {
    /// Serves the MCP command channel from inside the running game.
    struct McpPlugin {
        wiring: Wiring,
    }
}

impl McpPlugin {
    crate::support_item! {
        /// Listen on `port`, or open no listener at all when there is none.
        #[must_use]
        const fn from_port(port: Option<McpPort>) -> Self {
            let wiring = match port {
                Some(port) => Wiring::Listener {
                    port,
                    timeouts: NetTimeouts::DEFAULT,
                },
                None => Wiring::Disabled,
            };
            Self { wiring }
        }
    }

    crate::support_item! {
        /// Build the wiring `GDTF_MCP_PORT` asks for; no variable means no listener.
        #[must_use]
        fn from_env() -> Self {
            Self::from_port(port_from_env())
        }
    }

    /// Build a plugin that reads from an in-process inbox (tests).
    #[cfg(feature = "headless_test")]
    #[must_use]
    pub const fn with_channels(inbox: mpsc::Receiver<IncomingRequest>) -> Self {
        Self {
            wiring: Wiring::Channels {
                inbox: std::sync::Mutex::new(Some(inbox)),
            },
        }
    }

    /// Bind a loopback listener on `port` for tests, running with [`NetTimeouts::NO_IDLE_REAP`].
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    #[cfg(feature = "headless_test")]
    pub fn listening(port: McpPort) -> std::io::Result<(Self, McpPort)> {
        Self::listening_with(port, NetTimeouts::NO_IDLE_REAP)
    }

    /// Bind a loopback listener on `port` for tests, running with `timeouts`.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    #[cfg(feature = "headless_test")]
    pub fn listening_with(
        port: McpPort,
        timeouts: NetTimeouts,
    ) -> std::io::Result<(Self, McpPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener: std::sync::Mutex::new(Some(listener)),
                timeouts,
            },
        };
        Ok((plugin, bound))
    }
}

fn serve(app: &mut App, listener: TcpListener, timeouts: NetTimeouts) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    thread::spawn(move || run_listener(listener, tx, timeouts, hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    register_transport(app);
    register_consumers(app);
    register_present(app);
}

impl Default for McpPlugin {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for McpPlugin {
    fn build(&self, app: &mut App) {
        match &self.wiring {
            Wiring::Disabled => {}
            Wiring::Listener { port, timeouts } => {
                let Ok((listener, bound)) = bind_listener(*port) else {
                    error!(
                        port = **port,
                        "mcp: failed to bind the loopback listener — MCP channel OFF"
                    );
                    return;
                };
                info!(
                    port = *bound,
                    "mcp: ON (dev) — loopback MCP control channel listening"
                );
                serve(app, listener, *timeouts);
            }
            #[cfg(feature = "headless_test")]
            Wiring::Bound { listener, timeouts } => {
                let Some(listener) = listener.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                serve(app, listener, *timeouts);
            }
            #[cfg(feature = "headless_test")]
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

//! Net QA listener plugin for the content editor.

use std::{
    io,
    net::TcpListener,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use gdtf_net_qa_transport::{IncomingRequest, NetInbox, bind_listener, run_listener};
use gdtf_qa_protocol::{ports::NetQaPort, timeouts::NetTimeouts};

use super::{
    config::{DEFAULT_EDITOR_PORT, editor_hello_facts},
    router::route_editor_requests,
    schedule::EditorNetQaSystems,
};
use crate::EditorState;

enum Wiring {
    Listener {
        port:     NetQaPort,
        timeouts: NetTimeouts,
    },
    Bound {
        listener: Mutex<Option<TcpListener>>,
        timeouts: NetTimeouts,
    },
}

/// Plugin that optionally opens the editor net QA control channel.
pub struct NetQaEditorPlugin {
    wiring: Wiring,
}

impl NetQaEditorPlugin {
    /// Build the default wiring: debug builds listen on the shared editor QA port.
    #[must_use]
    pub const fn from_env() -> Self {
        Self {
            wiring: Wiring::Listener {
                port:     DEFAULT_EDITOR_PORT,
                timeouts: NetTimeouts::DEFAULT,
            },
        }
    }

    /// Bind a loopback listener on `port` for tests.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    pub fn listening(port: NetQaPort) -> io::Result<(Self, NetQaPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener: Mutex::new(Some(listener)),
                timeouts: NetTimeouts::DEFAULT,
            },
        };
        Ok((plugin, bound))
    }
}

impl Default for NetQaEditorPlugin {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for NetQaEditorPlugin {
    fn build(&self, app: &mut App) {
        match &self.wiring {
            Wiring::Listener { port, timeouts } => {
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
                serve(app, listener, *timeouts);
            }
            Wiring::Bound { listener, timeouts } => {
                let Some(listener) = listener.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                serve(app, listener, *timeouts);
            }
        }
    }
}

fn serve(app: &mut App, listener: TcpListener, timeouts: NetTimeouts) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    thread::spawn(move || run_listener(listener, tx, timeouts, editor_hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    app.configure_sets(
        Update,
        EditorNetQaSystems::Gather.run_if(resource_exists::<State<EditorState>>),
    );
    app.add_systems(
        Update,
        route_editor_requests.in_set(EditorNetQaSystems::Gather),
    );
}

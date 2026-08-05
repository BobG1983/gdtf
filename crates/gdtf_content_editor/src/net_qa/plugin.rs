//! Net QA listener plugin for the content editor.

use std::{
    io,
    net::TcpListener,
    path::PathBuf,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use gdtf_net_qa_transport::{
    DEFAULT_IO_TIMEOUT, IncomingRequest, NetInbox, NetIoTimeout, NetQaPort, bind_listener,
    run_listener,
};
use gdtf_screenshot::{CapturePipelinePlugin, CapturePresentPlugin, PresentSystems, ShotDir};

use super::{
    config::{DEFAULT_EDITOR_PORT, EDITOR_QA_SHOT_DIR, editor_hello_facts},
    present::retarget_editor_camera_to_offscreen,
    router::route_editor_requests,
    schedule::EditorNetQaSystems,
};
use crate::EditorState;

enum Wiring {
    Listener {
        port:       NetQaPort,
        io_timeout: NetIoTimeout,
    },
    Bound {
        listener:   Mutex<Option<TcpListener>>,
        io_timeout: NetIoTimeout,
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
                port:       DEFAULT_EDITOR_PORT,
                io_timeout: DEFAULT_IO_TIMEOUT,
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
                listener:   Mutex::new(Some(listener)),
                io_timeout: DEFAULT_IO_TIMEOUT,
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

fn serve(app: &mut App, listener: TcpListener, io_timeout: NetIoTimeout) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    thread::spawn(move || run_listener(listener, tx, io_timeout, editor_hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    if !app.is_plugin_added::<CapturePipelinePlugin<()>>() {
        app.add_plugins(CapturePipelinePlugin::<()>::new());
    }
    app.insert_resource(ShotDir::new(PathBuf::from(EDITOR_QA_SHOT_DIR)));
    app.add_plugins(CapturePresentPlugin);
    app.add_systems(
        Update,
        retarget_editor_camera_to_offscreen.in_set(PresentSystems),
    );
    app.configure_sets(
        Update,
        EditorNetQaSystems::Gather.run_if(resource_exists::<State<EditorState>>),
    );
    app.add_systems(
        Update,
        route_editor_requests.in_set(EditorNetQaSystems::Gather),
    );
}

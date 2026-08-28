//! Net QA listener plugin for the content editor.

use std::{
    io,
    net::TcpListener,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use gdtf_net_qa_transport::{IncomingRequest, NetInbox, bind_listener, run_listener};
use gdtf_qa_command::dispatch::QaCommandSystems;
use gdtf_qa_protocol::{ports::NetQaPort, timeouts::NetTimeouts};
use gdtf_screenshot::{
    CapturePipelinePlugin, CapturePresentPlugin, ShotDir, ShotDirName, WindowCapturePlugin,
};

use super::{
    commands::{EditorShotResponder, register_editor_commands},
    config::{editor_hello_facts, editor_port_from_env},
    router::route_editor_requests,
    schedule::EditorNetQaSystems,
};
use crate::EditorState;

/// Directory under the workspace `target/` the editor's own shots land in.
const EDITOR_QA_SHOT_DIR: &str = "qa_screenshots_editor";

enum Wiring {
    Listener {
        port:     NetQaPort,
        timeouts: NetTimeouts,
    },
    Bound {
        listener: Mutex<Option<TcpListener>>,
    },
}

/// Plugin that optionally opens the editor net QA control channel.
pub struct NetQaEditorPlugin {
    wiring: Wiring,
}

impl NetQaEditorPlugin {
    /// Build the default wiring: the listen port comes from `GDTF_EDITOR_NET_QA_PORT`,
    /// falling back to `DEFAULT_EDITOR_PORT`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            wiring: Wiring::Listener {
                port:     editor_port_from_env(),
                timeouts: NetTimeouts::DEFAULT,
            },
        }
    }

    /// Bind a loopback listener on `port` for tests, running with [`NetTimeouts::NO_IDLE_REAP`].
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    pub fn listening(port: NetQaPort) -> io::Result<(Self, NetQaPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener: Mutex::new(Some(listener)),
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
            Wiring::Bound { listener } => {
                let Some(listener) = listener.lock().ok().and_then(|mut guard| guard.take()) else {
                    return;
                };
                serve(app, listener, NetTimeouts::NO_IDLE_REAP);
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
        route_editor_requests.in_set(QaCommandSystems::Route),
    );
    register_editor_capture(app);
    register_editor_commands(app);
}

// The editor's own shot directory, then the three plugins one editor capture runs through.
fn register_editor_capture(app: &mut App) {
    app.insert_resource(ShotDir::under_workspace_target(&ShotDirName::new(
        EDITOR_QA_SHOT_DIR,
    )));
    if !app.is_plugin_added::<CapturePresentPlugin>() {
        app.add_plugins(CapturePresentPlugin);
    }
    if !app.is_plugin_added::<WindowCapturePlugin>() {
        app.add_plugins(WindowCapturePlugin);
    }
    if !app.is_plugin_added::<CapturePipelinePlugin<EditorShotResponder>>() {
        app.add_plugins(CapturePipelinePlugin::<EditorShotResponder>::new());
    }
}

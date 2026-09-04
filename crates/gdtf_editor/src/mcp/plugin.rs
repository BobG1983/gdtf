//! MCP listener plugin for the content editor.

use std::{
    io,
    net::TcpListener,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use cobalt_mcp_command::dispatch::QaCommandSystems;
use cobalt_mcp_protocol::{ports::McpPort, timeouts::NetTimeouts};
use cobalt_mcp_transport::{IncomingRequest, NetInbox, bind_listener, run_listener};
use cobalt_screenshot::{
    CapturePipelinePlugin, CapturePresentPlugin, ShotDir, ShotDirName, WindowCapturePlugin,
};

use super::{
    commands::{EditorShotResponder, register_editor_commands},
    config::{editor_hello_facts, editor_port_from_env},
    router::route_editor_requests,
    schedule::EditorMcpSystems,
};
use crate::EditorState;

/// Directory under the workspace `target/` the editor's own shots land in.
const EDITOR_QA_SHOT_DIR: &str = "qa_screenshots_editor";

enum Wiring {
    Listener {
        port:     McpPort,
        timeouts: NetTimeouts,
    },
    Bound {
        listener: Mutex<Option<TcpListener>>,
    },
}

/// Plugin that optionally opens the editor MCP control channel.
pub struct McpEditorPlugin {
    wiring: Wiring,
}

impl McpEditorPlugin {
    /// Build the default wiring: the listen port comes from `EDITOR_MCP_PORT`,
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
    pub fn listening(port: McpPort) -> io::Result<(Self, McpPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener: Mutex::new(Some(listener)),
            },
        };
        Ok((plugin, bound))
    }
}

impl Default for McpEditorPlugin {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for McpEditorPlugin {
    fn build(&self, app: &mut App) {
        match &self.wiring {
            Wiring::Listener { port, timeouts } => {
                let Ok((listener, bound)) = bind_listener(*port) else {
                    error!(
                        port = **port,
                        "editor mcp: failed to bind the loopback listener — QA channel OFF"
                    );
                    return;
                };
                info!(
                    port = *bound,
                    "editor mcp: ON (dev) — loopback QA control channel listening"
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
        EditorMcpSystems::Gather.run_if(resource_exists::<State<EditorState>>),
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

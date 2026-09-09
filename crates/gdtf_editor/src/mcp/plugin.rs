//! MCP listener plugin for the content editor.

use std::{
    io,
    net::TcpListener,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use cobalt_mcp_host::{
    IncomingRequest, NetInbox, bind_listener, dispatch::McpCommandSystems, run_listener,
};
use cobalt_mcp_protocol::{ports::McpPort, timeouts::NetTimeouts};
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
const EDITOR_MCP_SHOT_DIR: &str = "qa_screenshots_editor";

enum Wiring {
    Disabled,
    Listener {
        port:     McpPort,
        timeouts: NetTimeouts,
    },
    Bound {
        listener: Mutex<Option<TcpListener>>,
        timeouts: NetTimeouts,
    },
}

/// Plugin that optionally opens the editor MCP control channel.
pub struct McpEditorPlugin {
    wiring: Wiring,
}

impl McpEditorPlugin {
    /// Listen on `port`, or open no listener at all when there is none.
    #[must_use]
    pub const fn from_port(port: Option<McpPort>) -> Self {
        let wiring = match port {
            Some(port) => Wiring::Listener {
                port,
                timeouts: NetTimeouts::DEFAULT,
            },
            None => Wiring::Disabled,
        };
        Self { wiring }
    }

    /// Build the wiring `EDITOR_MCP_PORT` asks for; no variable means no listener.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_port(editor_port_from_env())
    }

    /// Bind a loopback listener on `port` for tests, running with [`NetTimeouts::NO_IDLE_REAP`].
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    pub fn listening(port: McpPort) -> io::Result<(Self, McpPort)> {
        Self::listening_with(port, NetTimeouts::NO_IDLE_REAP)
    }

    /// Bind a loopback listener on `port` for tests, running with `timeouts`.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the listener cannot bind.
    pub fn listening_with(port: McpPort, timeouts: NetTimeouts) -> io::Result<(Self, McpPort)> {
        let (listener, bound) = bind_listener(port)?;
        let plugin = Self {
            wiring: Wiring::Bound {
                listener: Mutex::new(Some(listener)),
                timeouts,
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
            Wiring::Disabled => {}
            Wiring::Listener { port, timeouts } => {
                let Ok((listener, bound)) = bind_listener(*port) else {
                    error!(
                        port = **port,
                        "editor mcp: failed to bind the loopback listener — MCP channel OFF"
                    );
                    return;
                };
                info!(
                    port = *bound,
                    "editor mcp: ON (dev) — loopback MCP control channel listening"
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
        EditorMcpSystems::Gather.run_if(resource_exists::<State<EditorState>>),
    );
    app.add_systems(
        Update,
        route_editor_requests.in_set(McpCommandSystems::Route),
    );
    register_editor_capture(app);
    register_editor_commands(app);
}

// The editor's own shot directory, then the three plugins one editor capture runs through.
fn register_editor_capture(app: &mut App) {
    app.insert_resource(ShotDir::under_workspace_target(&ShotDirName::new(
        EDITOR_MCP_SHOT_DIR,
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

#[cfg(test)]
mod test {
    use bevy::app::App;
    use cobalt_mcp_host::NetInbox;

    use super::McpEditorPlugin;

    #[test]
    fn no_port_opens_no_listener() {
        let mut app = App::new();
        app.add_plugins(McpEditorPlugin::from_port(None));

        assert!(
            app.world().get_resource::<NetInbox>().is_none(),
            "the inbox only reaches the world from the serve path, so its presence means a \
             listener thread started for a host that was given no port"
        );
    }
}

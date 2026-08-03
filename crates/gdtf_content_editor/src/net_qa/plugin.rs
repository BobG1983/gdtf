//! 1. **`cfg(all(debug_assertions, feature = "net_qa"))`.** The wiring site (`crate::app`)
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
    config::editor_hello_facts,
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

enum Wiring {
        Disabled,
        Listener {
                port:       NetQaPort,
                io_timeout: NetIoTimeout,
    },
                Bound {
                listener:   Mutex<Option<TcpListener>>,
                io_timeout: NetIoTimeout,
    },
}

pub struct NetQaEditorPlugin {
        wiring: Wiring,
}

impl NetQaEditorPlugin {
                            /// `cfg(all(debug_assertions, feature = "net_qa"))` — the ONLY one the editor binary ever
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

                                        /// module it compiles only under `cfg(all(debug_assertions, feature = "net_qa"))`, and the
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

fn serve(app: &mut App, listener: TcpListener, io_timeout: NetIoTimeout) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    thread::spawn(move || run_listener(listener, tx, io_timeout, editor_hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    app.init_resource::<PendingQueue<EditorScreenshotPayload>>();
    app.init_resource::<EditorInFlightShots>();
    app.init_resource::<EditorShotSequence>();
    app.init_resource::<EditorQaShotDir>();
    app.init_resource::<EditorShotSettle>();
    app.init_resource::<EditorShotPollBudget>();
    app.init_resource::<EditorShotSource>();
    app.add_plugins(EditorCapturePresentPlugin);
    app.configure_sets(
        Update,
        EditorNetQaSystems::Gather.run_if(resource_exists::<State<EditorState>>),
    );
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

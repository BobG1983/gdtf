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

enum Wiring {
        Disabled,
        Listener {
                port:       NetQaPort,
                io_timeout: NetIoTimeout,
    },
                #[cfg(feature = "test-support")]
    Channels {
                inbox: std::sync::Mutex<Option<mpsc::Receiver<IncomingRequest>>>,
    },
                #[cfg(feature = "test-support")]
    Bound {
                listener:   std::sync::Mutex<Option<TcpListener>>,
                io_timeout: NetIoTimeout,
    },
}

crate::support_item! {
        struct NetQaPlugin {
                wiring: Wiring,
    }
}

impl NetQaPlugin {
    crate::support_item! {
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

                        #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn with_channels(inbox: mpsc::Receiver<IncomingRequest>) -> Self {
        Self {
            wiring: Wiring::Channels {
                inbox: std::sync::Mutex::new(Some(inbox)),
            },
        }
    }

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

fn serve(app: &mut App, listener: TcpListener, io_timeout: NetIoTimeout) {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    thread::spawn(move || run_listener(listener, tx, io_timeout, hello_facts()));
    app.insert_resource(NetInbox::new(rx));
    register_transport(app);
    register_consumers(app);
    app.add_plugins(CapturePresentPlugin);
}

impl Default for NetQaPlugin {
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

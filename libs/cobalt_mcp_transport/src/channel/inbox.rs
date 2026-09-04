//! Bevy resource that drains incoming QA requests from the listener thread.

use std::sync::{Mutex, mpsc::Receiver};

use bevy::prelude::*;

use super::IncomingRequest;

/// Inbox of requests delivered from the TCP listener.
#[derive(Resource)]
pub struct NetInbox(Mutex<Receiver<IncomingRequest>>);

impl NetInbox {
    /// Wrap a receiver.
    #[must_use]
    pub const fn new(rx: Receiver<IncomingRequest>) -> Self {
        Self(Mutex::new(rx))
    }

    /// Drain all currently queued requests (non-blocking).
    #[must_use]
    pub fn drain(&self) -> Vec<IncomingRequest> {
        let Ok(rx) = self.0.lock() else {
            return Vec::new();
        };
        rx.try_iter().collect()
    }
}

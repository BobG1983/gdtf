//! The Bevy-side receiving end of the listener → host request channel (GTW-736).

use std::sync::{Mutex, mpsc::Receiver};

use bevy::prelude::*;

use super::IncomingRequest;

/// The host-side inbox: the receiving end of the listener → router request channel.
///
/// Holds a [`Mutex`]-wrapped [`Receiver`] because `std::sync::mpsc::Receiver` is `!Sync`
/// and a Bevy [`Resource`] must be `Sync`; the host's router locks it briefly each frame
/// to drain whatever the listener pushed. Named-newtype resource with a private inner
/// (no-bare-types framework plumbing).
#[derive(Resource)]
pub struct NetInbox(Mutex<Receiver<IncomingRequest>>);

impl NetInbox {
    /// Wrap the receiving end of the request channel.
    #[must_use]
    pub const fn new(rx: Receiver<IncomingRequest>) -> Self {
        Self(Mutex::new(rx))
    }

    /// Drain every request the listener has pushed since the last frame.
    ///
    /// Locks the inner [`Mutex`] briefly and non-blockingly pulls until the channel is
    /// empty. A poisoned lock (a listener thread panicked mid-send — it never does) or a
    /// disconnected channel yields nothing rather than panicking (bevy-traps: no panic in
    /// the happy path).
    #[must_use]
    pub fn drain(&self) -> Vec<IncomingRequest> {
        let Ok(rx) = self.0.lock() else {
            return Vec::new();
        };
        rx.try_iter().collect()
    }
}

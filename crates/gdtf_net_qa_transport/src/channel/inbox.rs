use std::sync::{Mutex, mpsc::Receiver};

use bevy::prelude::*;

use super::IncomingRequest;

#[derive(Resource)]
pub struct NetInbox(Mutex<Receiver<IncomingRequest>>);

impl NetInbox {
        #[must_use]
    pub const fn new(rx: Receiver<IncomingRequest>) -> Self {
        Self(Mutex::new(rx))
    }

                            #[must_use]
    pub fn drain(&self) -> Vec<IncomingRequest> {
        let Ok(rx) = self.0.lock() else {
            return Vec::new();
        };
        rx.try_iter().collect()
    }
}

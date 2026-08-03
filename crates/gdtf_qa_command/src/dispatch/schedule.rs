//! System sets for routing and claiming QA commands.

use bevy::prelude::*;

/// Ordered sets used when registering command systems.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QaCommandSystems {
    /// Route inbound protocol requests into the inbox.
    Route,
    /// Claim inbox rows into typed pending queues.
    Claim,
}

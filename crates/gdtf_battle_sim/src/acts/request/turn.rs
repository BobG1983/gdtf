//! Request to end the current faction's turn.

use bevy::prelude::Message;

/// End-turn signal.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndTurnRequested;

use bevy::prelude::Message;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndTurnRequested;

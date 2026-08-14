//! Net QA system sets for the editor.

use bevy::prelude::*;

/// System sets for the editor net QA channel.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorNetQaSystems {
    /// Gather, route, claim, and answer incoming requests.
    Gather,
}

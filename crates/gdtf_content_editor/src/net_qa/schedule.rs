//! Net QA system sets for the editor.

use bevy::prelude::*;

/// System sets for the editor net QA channel.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorNetQaSystems {
    /// Present captured frames.
    Present,
    /// Gather and route incoming requests.
    Gather,
}

//! Classified consequence pop and the `ConsequenceFct` trait.

use bevy::prelude::{Color, Message};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::{CombatText, FctEmphasis};

/// Where a pop should appear in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopAnchor {
    /// Fixed cell carried on the signal.
    Carried(CellLevel),
    /// Follow a ganger entity's current position.
    GangerPosition(bevy::prelude::Entity),
}

/// Text, color, weight, and anchor for one consequence pop.
#[derive(Debug, Clone)]
pub struct ConsequencePop {
    text: CombatText,
    color: Color,
    emphasis: FctEmphasis,
    anchor: PopAnchor,
}

impl ConsequencePop {
    /// Normal-weight pop.
    #[must_use]
    pub const fn new(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
            anchor,
        }
    }

    /// Bold-weight pop.
    #[must_use]
    pub const fn new_bold(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
            anchor,
        }
    }

    /// Display text.
    #[must_use]
    pub const fn text(&self) -> &CombatText {
        &self.text
    }

    /// Tint color.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Font emphasis.
    #[must_use]
    pub const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }

    /// World anchor.
    #[must_use]
    pub const fn anchor(&self) -> PopAnchor {
        self.anchor
    }
}

/// Maps a played consequence signal to a floating combat pop.
pub trait ConsequenceFct: Send + Sync + 'static {
    /// Message type this family reads.
    type Signal: Message + Clone;

    /// Classify a signal into a pop.
    fn classify(signal: &Self::Signal) -> ConsequencePop;
}

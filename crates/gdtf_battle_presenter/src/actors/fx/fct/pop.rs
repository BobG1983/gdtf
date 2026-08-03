use bevy::prelude::{Color, Message};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::{CombatText, FctEmphasis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopAnchor {
        Carried(CellLevel),
                        GangerPosition(bevy::prelude::Entity),
}

#[derive(Debug, Clone)]
pub struct ConsequencePop {
        text:     CombatText,
        color:    Color,
        emphasis: FctEmphasis,
        anchor:   PopAnchor,
}

impl ConsequencePop {
        #[must_use]
    pub const fn new(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
            anchor,
        }
    }

            #[must_use]
    pub const fn new_bold(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
            anchor,
        }
    }

        #[must_use]
    pub const fn text(&self) -> &CombatText {
        &self.text
    }

            #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

        #[must_use]
    pub const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }

        #[must_use]
    pub const fn anchor(&self) -> PopAnchor {
        self.anchor
    }
}

pub trait ConsequenceFct: Send + Sync + 'static {
                        type Signal: Message + Clone;

                fn classify(signal: &Self::Signal) -> ConsequencePop;
}

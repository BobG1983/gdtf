use bevy::prelude::Color;

use super::super::text::{CombatText, FctEmphasis};

#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
        text:     CombatText,
        color:    Color,
        emphasis: FctEmphasis,
}

impl LogLine {
        #[must_use]
    pub(super) const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

            #[must_use]
    pub(super) const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
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
}

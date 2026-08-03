use bevy::prelude::*;

use super::super::text::{CombatText, FctEmphasis};

#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct ClassifiedPop {
        pub(super) text:     CombatText,
            pub(super) color:    Color,
            pub(super) emphasis: FctEmphasis,
}

impl ClassifiedPop {
        pub(super) const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

            pub(super) const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
        }
    }

            pub(in crate::actors::fx) const fn text(&self) -> &CombatText {
        &self.text
    }

            pub(in crate::actors::fx) const fn color(&self) -> Color {
        self.color
    }

            pub(in crate::actors::fx) const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }
}

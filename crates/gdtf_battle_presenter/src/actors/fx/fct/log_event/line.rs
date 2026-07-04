//! The rendered combat-log LINE value — [`LogLine`], one scroll-up-and-fade line's text,
//! swatch, and emphasis weight.

use bevy::prelude::Color;

use super::super::text::{CombatText, FctEmphasis};

/// One rendered combat-log line — its text, its valence color, and its emphasis weight.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color, FctEmphasis)` tuple): mirrors
/// the FCT [`ClassifiedPop`](super::super::reader::ClassifiedPop) shape but is the log
/// layer's unit. Reuses the existing FCT vocabulary: [`CombatText`] for the string, the
/// [`FctValence`](super::super::palette::FctValence) palette via
/// [`valence_color`](super::super::palette::valence_color) for the color, and
/// [`FctEmphasis`] for the weight (the lethal DOWN / DEAD / dies line rides
/// [`FctEmphasis::Bold`]). The [`Color`](bevy::prelude::Color) is framework plumbing (the
/// swatch the renderer draws), the only bare type the no-bare-types rule permits here.
///
/// `pub`: the app-side appender renders the lines — read through the accessors (the fields
/// stay private so a line is only built through
/// [`classify_log_event`](super::classify_log_event), never field-assembled outside).
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    /// The combat-text string this line renders.
    text:     CombatText,
    /// The valence swatch the line is drawn in.
    color:    Color,
    /// The styling weight the line is drawn at ([`FctEmphasis::Bold`] for the lethal line).
    emphasis: FctEmphasis,
}

impl LogLine {
    /// Build an ordinary (body-weight) log line from its text + valence color.
    #[must_use]
    pub(super) const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

    /// Build an EMPHASIZED (bold) log line from its text + valence color — the lethal
    /// DOWN / DEAD / dies line, the heaviest in the blood family.
    #[must_use]
    pub(super) const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
        }
    }

    /// The line's combat-text string.
    #[must_use]
    pub const fn text(&self) -> &CombatText {
        &self.text
    }

    /// The line's valence swatch — the [`Color`](bevy::prelude::Color) the renderer draws.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// The line's styling weight ([`FctEmphasis::Bold`] for the lethal line, else
    /// [`FctEmphasis::Normal`]).
    #[must_use]
    pub const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }
}

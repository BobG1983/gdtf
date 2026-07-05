//! Text-style derivation shared by spawn, repaint, and the sub-line: the label /
//! sub-line fonts, the bold-on-active weight channel, and the font constants.

use bevy::{
    prelude::*,
    text::{FontSize, FontWeight, TextFont},
};

/// The [`FontWeight`](bevy::text::FontWeight) of a segment label by active-ness — the
/// color-blind-safe second channel (bold on active, normal otherwise).
pub(super) const fn active_weight(is_active: bool) -> FontWeight {
    if is_active {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    }
}

/// The label [`TextFont`](bevy::text::TextFont) for a segment by active-ness.
pub(super) fn segment_font(is_active: bool) -> TextFont {
    TextFont {
        font_size: FontSize::Px(SEGMENT_FONT_PT),
        weight: active_weight(is_active),
        ..default()
    }
}

/// The [`TextFont`](bevy::text::TextFont) for a segment's OPTIONAL sub-line (GTW-303): the
/// smaller [`SEGMENT_SUB_FONT_PT`] at normal weight — quieter than the bold-on-active label.
pub(super) fn sub_line_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(SEGMENT_SUB_FONT_PT),
        weight: FontWeight::NORMAL,
        ..default()
    }
}

/// The DIMMED color of a segment's sub-line, derived from the segment's `base_text` color
/// (GTW-303): the same hue at [`SEGMENT_SUB_ALPHA`] opacity, so the sub-line reads as a
/// quieter second line that matches the palette without the caller re-passing a color.
pub(super) fn sub_line_color(base_text: Color) -> Color {
    base_text.with_alpha(base_text.alpha() * SEGMENT_SUB_ALPHA)
}

/// The segment label font size, in typographic points.
const SEGMENT_FONT_PT: f32 = 16.0;

/// The segment SUB-LINE font size, in typographic points (GTW-303): ~11 pt — distinctly
/// smaller than the [`SEGMENT_FONT_PT`] label so the second line reads as a quiet annotation
/// (the TU cost under the firemode name). Font size in pt is the ONE permitted px exception to
/// the relative-units rule (`ui-responsive-not-px`), matching the label const.
pub(super) const SEGMENT_SUB_FONT_PT: f32 = 11.0;

/// The opacity MULTIPLIER applied to a segment's `base_text` color to dim its sub-line
/// (GTW-303): 0.7 — visibly dimmer than the label without becoming unreadable.
const SEGMENT_SUB_ALPHA: f32 = 0.7;

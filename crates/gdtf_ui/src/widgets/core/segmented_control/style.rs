use bevy::{
    prelude::*,
    text::{FontSize, FontWeight, TextFont},
};

pub(super) const fn active_weight(is_active: bool) -> FontWeight {
    if is_active {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    }
}

pub(super) fn segment_font(is_active: bool) -> TextFont {
    TextFont {
        font_size: FontSize::Px(SEGMENT_FONT_PT),
        weight: active_weight(is_active),
        ..default()
    }
}

pub(super) fn sub_line_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(SEGMENT_SUB_FONT_PT),
        weight: FontWeight::NORMAL,
        ..default()
    }
}

pub(super) fn sub_line_color(base_text: Color) -> Color {
    base_text.with_alpha(base_text.alpha() * SEGMENT_SUB_ALPHA)
}

const SEGMENT_FONT_PT: f32 = 16.0;

pub(super) const SEGMENT_SUB_FONT_PT: f32 = 11.0;

const SEGMENT_SUB_ALPHA: f32 = 0.7;

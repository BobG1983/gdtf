use bevy::color::Color;

pub(in crate::states::running::game::battlescape) const TU_REMAINING: Color =
    Color::srgb(0.36, 0.78, 0.92);

pub(in crate::states::running::game::battlescape) const TU_LOST: Color =
    Color::srgb(0.12, 0.14, 0.16);

pub(in crate::states::running::game::battlescape) const HP_REMAINING: Color =
    Color::srgb(0.30, 0.78, 0.36);

pub(in crate::states::running::game::battlescape) const HP_LOST: Color =
    Color::srgb(0.70, 0.18, 0.18);

pub(in crate::states::running::game::battlescape) const WOUNDS_REMAINING: Color =
    Color::srgb(0.92, 0.82, 0.24);

pub(in crate::states::running::game::battlescape) const WOUNDS_LOST: Color =
    Color::srgb(0.12, 0.14, 0.16);

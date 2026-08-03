use bevy::prelude::*;
use gdtf_battle_sim::severity::Severity;

const DAMAGE_RED: Color = Color::srgb(0.90, 0.13, 0.10);

const WOUND_AMBER: Color = Color::srgb(0.95, 0.70, 0.15);

const WOUND_AMBER_DEEP: Color = Color::srgb(0.96, 0.42, 0.08);

const NEUTRAL_GREY: Color = Color::srgb(0.72, 0.72, 0.74);

const SUPPRESSED_BLUE_GREY: Color = Color::srgb(0.45, 0.55, 0.72);

const DOT_TOXIC_GREEN: Color = Color::srgb(0.35, 0.82, 0.20);

const FIELD_HAZARD_ORANGE: Color = Color::srgb(0.95, 0.50, 0.10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FctValence {
        Damage,
                Status,
        Neutral,
                Lethal,
                Suppressed,
                Dot,
                    Field,
}

#[must_use]
pub const fn valence_color(valence: FctValence) -> Color {
    match valence {
        FctValence::Damage | FctValence::Lethal => DAMAGE_RED,
        FctValence::Status => WOUND_AMBER,
        FctValence::Neutral => NEUTRAL_GREY,
        FctValence::Suppressed => SUPPRESSED_BLUE_GREY,
        FctValence::Dot => DOT_TOXIC_GREEN,
        FctValence::Field => FIELD_HAZARD_ORANGE,
    }
}

#[must_use]
pub fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::None => NEUTRAL_GREY,
        Severity::Fatal => DAMAGE_RED,
        wound => {
                        const MIN_WOUND_RANK: f32 = 1.0;
                                    const WOUND_RANK_SPAN: f32 = 2.0;
            let t = ((f32::from(*wound.rank()) - MIN_WOUND_RANK) / WOUND_RANK_SPAN).clamp(0.0, 1.0);
            WOUND_AMBER.mix(&WOUND_AMBER_DEEP, t)
        }
    }
}

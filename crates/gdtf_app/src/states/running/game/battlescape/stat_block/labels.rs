use gdtf_battle_sim::{
    armor::BodyPart,
    ganger::GangerName,
    inflicted_wound::InflictedWound,
    prelude::{Faction, Stance, StanceKind},
    severity::Severity,
};

pub(in crate::states::running::game::battlescape) const NAMELESS: &str = "???";

#[must_use]
pub(in crate::states::running::game::battlescape) fn name_label(
    name: Option<&GangerName>,
) -> String {
    name.map_or(NAMELESS, |n| n).to_owned()
}

#[must_use]
pub(in crate::states::running::game::battlescape) fn faction_label(faction: Faction) -> String {
    format!("Gang {}", *faction)
}

#[must_use]
pub(in crate::states::running::game::battlescape) fn stance_label(stance: Stance) -> String {
    let word = match *stance {
        StanceKind::Standing => "Standing",
        StanceKind::Crouching => "Crouching",
        StanceKind::Prone => "Prone",
    };
    format!("Stance: {word}")
}

#[must_use]
pub(in crate::states::running::game::battlescape) fn wound_label(wound: InflictedWound) -> String {
    format!(
        "{} — {}",
        severity_word(wound.tier),
        body_part_word(wound.location)
    )
}

const fn severity_word(tier: Severity) -> &'static str {
    match tier {
        Severity::None => "Graze",
        Severity::Minor => "Minor",
        Severity::Major => "Major",
        Severity::Critical => "Critical",
        Severity::Fatal => "Fatal",
    }
}

const fn body_part_word(part: BodyPart) -> &'static str {
    match part {
        BodyPart::Head => "Head",
        BodyPart::Torso => "Torso",
        BodyPart::LeftArm => "Left Arm",
        BodyPart::RightArm => "Right Arm",
        BodyPart::LeftLeg => "Left Leg",
        BodyPart::RightLeg => "Right Leg",
    }
}

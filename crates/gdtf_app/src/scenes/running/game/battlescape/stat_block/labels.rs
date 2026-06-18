//! Pure format helpers that render a ganger's typed components into the stat-block's
//! display strings (GTW-278 / GTW-274).
//!
//! Each helper takes the real typed sim component(s) and returns the line's [`String`] —
//! no [`App`](bevy::prelude::App), no `World`, no ECS — so they are unit-testable
//! directly and the builder/updater stay thin wiring. The vocabulary (Standing /
//! Crouching / Prone; Minor / Major / Critical; Head / Torso / arms / legs) matches the
//! sim's named domain enums. The status panel's removed life-state / weapon lines have
//! no helper here — those are deliberately gone (the contract: keep name / faction /
//! stance).

use gdtf_battle_sim::{
    BodyPart, Faction, GangerName, InflictedWound, Severity, Stance, StanceKind,
};

/// The name fallback shown when a ganger carries no [`GangerName`] — a placeholder so a
/// nameless ganger reads as unnamed without a panic.
///
/// A `const` so the no-name copy lives in one place (the status-panel `NAMELESS`
/// precedent).
pub(in crate::scenes::running::game::battlescape) const NAMELESS: &str = "???";

/// The **name title**: the ganger's [`GangerName`], or the [`NAMELESS`] fallback.
///
/// Taken as an `Option<&GangerName>` (defensive — gangers are named today, but a target
/// without the component must not panic). [`GangerName`] [`Deref`](std::ops::Deref)s to
/// its inner `&str`; by reference because it owns a `String` (not `Copy`).
#[must_use]
pub(in crate::scenes::running::game::battlescape) fn name_label(
    name: Option<&GangerName>,
) -> String {
    name.map_or(NAMELESS, |n| n).to_owned()
}

/// The **faction** line: the ganger's faction (gang) index, e.g. "Gang 1".
///
/// [`Faction`] derefs to its small gang index, taken by value (clippy
/// `trivially_copy_pass_by_ref` — a 1-byte `Copy` newtype).
#[must_use]
pub(in crate::scenes::running::game::battlescape) fn faction_label(faction: Faction) -> String {
    format!("Gang {}", *faction)
}

/// The **stance** line: the ganger's posture by its [`StanceKind`] name.
///
/// [`Stance`] derefs to the [`StanceKind`] posture; the rendered word matches the sim's
/// named vocabulary. Taken by value — `Stance` is a 1-byte `Copy` newtype.
#[must_use]
pub(in crate::scenes::running::game::battlescape) fn stance_label(stance: Stance) -> String {
    let word = match *stance {
        StanceKind::Standing => "Standing",
        StanceKind::Crouching => "Crouching",
        StanceKind::Prone => "Prone",
    };
    format!("Stance: {word}")
}

/// One **wound-name** line for an [`InflictedWound`]: `"{tier} — {location}"`
/// (e.g. "Minor — Left Arm").
///
/// The [`Severity`] tier and [`BodyPart`] location are rendered through this module's
/// display vocabulary (neither sim enum impls `Display` — the presentation policy lives
/// here, the `stance_label` precedent). [`InflictedWound`] is `Copy`, taken by value.
#[must_use]
pub(in crate::scenes::running::game::battlescape) fn wound_label(wound: InflictedWound) -> String {
    format!(
        "{} — {}",
        severity_word(wound.tier),
        body_part_word(wound.location)
    )
}

/// The display word for a wound-severity tier.
///
/// `Severity::None` is never a recorded wound (a graze records nothing), but it is
/// rendered defensively as "Graze" rather than panicking on an unexpected value.
const fn severity_word(tier: Severity) -> &'static str {
    match tier {
        Severity::None => "Graze",
        Severity::Minor => "Minor",
        Severity::Major => "Major",
        Severity::Critical => "Critical",
        Severity::Fatal => "Fatal",
    }
}

/// The display word for a struck body part — the §4 location vocabulary.
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

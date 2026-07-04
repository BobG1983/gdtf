//! The GTW-572 C6 state-change classify pins — the Q2 ruling's gain lines (fall / melee
//! damage / death / suppression / armor-broken) and the once-at-start affliction lines
//! (DOT / field / bleed). Each pins the EXACT phrasing + swatch POSITIVELY.

use gdtf_battle_sim::{DotDamage, HpDamage, StoreysFallen};

use super::super::{
    super::{
        palette::{FctValence, valence_color},
        text::FctEmphasis,
    },
    classify::classify_log_event,
    event::{CombatLogEvent, LogName},
};

/// A fall logs ONE line `"<actor> fell <N> storeys"` in the wound AMBER — and the storey
/// noun is singular for a one-storey drop.
#[test]
fn a_fall_logs_the_storey_count_in_wound_amber() {
    let two = CombatLogEvent::FallOccurred {
        actor:   LogName::new("Vex"),
        storeys: StoreysFallen::new(2),
    };
    let lines = classify_log_event(&two);
    assert_eq!(lines.len(), 1, "a fall is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex fell 2 storeys");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Wound),
        "the fall line is drawn the wound AMBER (a harm event)",
    );

    let one = CombatLogEvent::FallOccurred {
        actor:   LogName::new("Vex"),
        storeys: StoreysFallen::new(1),
    };
    assert_eq!(
        &**classify_log_event(&one)[0].text(),
        "Vex fell 1 storey",
        "a one-storey fall reads the singular noun",
    );
}

/// A connecting melee strike logs ONE line `"<attacker> struck <target> (-N)"` in the
/// damage RED — the number-bearing melee-damage line the `MeleeStruck` fact enables.
#[test]
fn a_melee_strike_logs_both_names_and_the_amount_in_damage_red() {
    let event = CombatLogEvent::MeleeStruck {
        attacker: LogName::new("Vex"),
        target:   LogName::new("Skar"),
        amount:   HpDamage::new(4),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a melee strike is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex struck Skar (-4)");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Damage),
        "the melee-damage line is drawn the damage RED",
    );
    assert_eq!(lines[0].emphasis(), FctEmphasis::Normal);
}

/// A terminal death logs ONE NAMED line `"<actor> dies"` in the lethal RED, BOLD — the
/// line mirror of the nameless DOWN / DEAD tag, covering the formerly log-invisible
/// DOT / field / bleed-out / melee / explode-cascade kills.
#[test]
fn a_death_logs_the_named_dies_line_bold_lethal() {
    let event = CombatLogEvent::OnDeathOccurred {
        actor: LogName::new("Vex"),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a death is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex dies");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Lethal),
        "the death line is drawn the lethal blood-red",
    );
    assert_eq!(
        lines[0].emphasis(),
        FctEmphasis::Bold,
        "the death line is BOLD (the heaviest line, like the DOWN / DEAD tag)",
    );
}

/// A fresh suppression logs ONE line `"<actor> is suppressed"` in the cowed suppression
/// blue-grey (the FCT tag's family).
#[test]
fn a_suppression_logs_the_named_suppressed_line() {
    let event = CombatLogEvent::SuppressionApplied {
        actor: LogName::new("Vex"),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a suppression is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex is suppressed");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Suppressed),
        "the suppression line is drawn the cowed blue-grey",
    );
}

/// An armor break logs ONE line `"<actor>: armor broken"` in the damage RED (the destroy
/// crossing reads heavier than wear, matching the FCT tag).
#[test]
fn an_armor_break_logs_the_named_armor_broken_line() {
    let event = CombatLogEvent::ArmorBroken {
        actor: LogName::new("Vex"),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "an armor break is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex: armor broken");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Damage),
        "the armor-broken line is drawn the damage RED",
    );
}

/// A DOT affliction START logs ONE line `"<actor> is afflicted (-N/turn)"` in the toxic
/// DOT green — the once-per-span line (the per-tick `DotTicked` has NO classify arm at all,
/// so a mid-affliction tick can never log).
#[test]
fn a_dot_affliction_start_logs_the_per_turn_line_in_dot_green() {
    let event = CombatLogEvent::DotAfflicted {
        actor:    LogName::new("Vex"),
        per_turn: DotDamage::new(2),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a DOT affliction start is exactly one line");
    assert_eq!(&**lines[0].text(), "Vex is afflicted (-2/turn)");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Dot),
        "the DOT affliction line is drawn the toxic DOT green",
    );
}

/// A field exposure START logs ONE line `"<actor> is caught in a hazard field"` in the
/// hazard field orange — the once-per-span line (the per-round `FieldTicked` has NO classify
/// arm at all).
#[test]
fn a_field_exposure_start_logs_the_hazard_line_in_field_orange() {
    let event = CombatLogEvent::FieldAfflicted {
        actor: LogName::new("Vex"),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a field exposure start is exactly one line");
    assert_eq!(&**lines[0].text(), "Vex is caught in a hazard field");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Field),
        "the field exposure line is drawn the hazard field orange",
    );
}

/// A bleed span START logs ONE line `"<actor> is bleeding"` in the wound AMBER — the
/// once-per-span line (the per-tick Bleeding has NO classify arm at all).
#[test]
fn a_bleed_span_start_logs_the_bleeding_line_in_wound_amber() {
    let event = CombatLogEvent::BleedStarted {
        actor: LogName::new("Vex"),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a bleed span start is exactly one line");
    assert_eq!(&**lines[0].text(), "Vex is bleeding");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Wound),
        "the bleeding line is drawn the wound AMBER (the FCT tag's family)",
    );
}

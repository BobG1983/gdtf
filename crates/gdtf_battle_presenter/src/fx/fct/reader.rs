//! GTW-302 (slice 3) / GTW-327 (slice 2): the [`ShotFired`] → floating-combat-text
//! CLASSIFICATION — the shared, reusable functions that turn each fired round's
//! already-computed [`HitReport`] into the ordered list of rise-and-fade pops the slice-2
//! primitive ([`spawn_floating_text`](super::text::spawn_floating_text)) draws, plus the
//! `(cell, level)` anchor those pops sit on.
//!
//! GTW-302 originally spawned the pops IMMEDIATELY when draining the [`ShotFired`] buffer (in a
//! `read_shot_fired_text` system here). That dumped a whole burst / full-auto volley's numbers
//! on the single drain frame even though the GTW-306 tracer projectiles
//! ([`spawn_shot_projectiles`](super::super::spawn_shot_projectiles)) fly STAGGERED, so the
//! numbers and the bolts desynced. GTW-327 slice 2 REPURPOSES this module: the immediate-spawn
//! system is gone; [`classify_report`] + [`anchor_cell`] are now the SHARED classification
//! [`spawn_shot_projectiles`] calls at projectile-spawn time, threading each round's
//! [`ClassifiedPop`] list + anchor THROUGH the staggered projectile → impact pipeline so each
//! shot's pops appear when THAT shot's impact lands (see
//! [`projectile`](super::super::projectile) / [`impact`](super::super::impact)). This also sets
//! up GTW-328's shared event → text layer (the classification is a clean, reusable seam).
//!
//! For each round [`classify_report`] CLASSIFIES its [`report`](gdtf_battle_sim::ShotFired::report)
//! into the Phase-1 combat events this slice covers, one pop per event:
//!
//! - **HP damage** (`report.applied.hit.hp_damage > 0`) — the numeric loss, e.g. `-7`, drawn
//!   the damage RED ([`FctValence::Damage`](super::palette::FctValence::Damage)).
//! - **Wound gained** (`report.applied.severity` is a real wounding tier AND `report.part`
//!   names a struck part) — `"<Part> <Tier>"`, e.g. `"Torso Major"`, drawn the AMBER ramp
//!   ([`severity_color`](super::palette::severity_color)).
//! - **Graze** (`report.applied.severity == Severity::None` on a ganger hit) — `"Grazed"`,
//!   drawn GREY ([`FctValence::Neutral`](super::palette::FctValence::Neutral)): HP loss but no
//!   Wound spent (resolution.md §6).
//! - **Penetration verdict** (`report.applied.hit.penetrating`) — `"Penetrated"` (GREY, it
//!   went through the armor) when `> 0`, else `"Deflected"` (AMBER, the armor soaked it).
//! - **DOWN / DEAD** (`report.applied.life_after`) — `"DOWN"` / `"DEAD"` drawn the lethal RED
//!   in [`FctEmphasis::Bold`](super::text::FctEmphasis::Bold) (the heaviest pop in the blood
//!   family — bold weight + a larger size, the contract's "RED bold"; the all-caps tag is a
//!   complementary cue, not a substitute).
//!
//! The AUX slice (4) covers the rest of the contract's Phase-1 list that is NOT derivable from
//! [`ShotFired`] alone — armor `"Armor -N"` / `"Armor Broken"`, reload `"Reloaded"` / `"Empty"`
//! / `"No TU"`, and bleeding — from the consequence messages
//! ([`ArmorBroken`](gdtf_battle_sim::ArmorBroken) / [`Bleeding`](gdtf_battle_sim::Bleeding)) or
//! a future reload signal. Those pops are NOT staggered (they ride their own one-shot
//! consequence messages, not the per-round projectile pipeline).
//!
//! Pure VIEW (ADR-0001): these functions only READ the message + look up the hit ganger's cell;
//! the SPAWN happens downstream (at the impact) and never reads any raw sim state by polling and
//! never writes the sim.

use bevy::prelude::*;
use gdtf_battle_sim::{
    BodyPart, Cell, HitReport, Level, LifeState, Position, Severity, ShotFired, ShotKind,
};

use super::{
    palette::{FctValence, severity_color, valence_color},
    text::{CombatText, FctEmphasis},
};

/// One ready-to-spawn floating-combat-text pop — the classified string, its valence color,
/// and its emphasis weight, before it is anchored at the hit cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color, FctEmphasis)` tuple):
/// [`classify_report`] builds the ordered list of pops one round's [`HitReport`] yields;
/// [`spawn_shot_projectiles`](super::super::spawn_shot_projectiles) threads that list THROUGH
/// the staggered projectile pipeline and [`animate_impact`](super::super::animate_impact)
/// anchors each at the round's cell with the next [`FctStackIndex`](super::text::FctStackIndex)
/// when the impact lands. The [`Color`](bevy::prelude::Color) is framework plumbing (the swatch
/// fed straight to the primitive), the only bare type the no-bare-types rule permits here.
///
/// `pub(in crate::fx)`: built here by [`classify_report`], consumed by the sibling
/// `projectile` / `impact` modules — read through the [`text`](Self::text) /
/// [`color`](Self::color) / [`emphasis`](Self::emphasis) accessors (the fields stay private so
/// a pop is only constructed through the classifier, never field-assembled outside).
#[derive(Debug, Clone)]
pub(in crate::fx) struct ClassifiedPop {
    /// The combat-text string this pop renders (a damage number, a wound tag, `"Grazed"`, …).
    text:     CombatText,
    /// The valence swatch the pop is drawn in (the damage RED / wound AMBER / neutral GREY /
    /// lethal RED family the event maps to).
    color:    Color,
    /// The styling weight the pop is drawn at — [`FctEmphasis::Bold`] for the lethal
    /// DOWN / DEAD tag (the contract's "RED bold"), [`FctEmphasis::Normal`] for every other.
    emphasis: FctEmphasis,
}

impl ClassifiedPop {
    /// Build an ordinary (body-weight) classified pop from its text + valence color.
    const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

    /// Build an EMPHASIZED (bold) classified pop from its text + valence color — the lethal
    /// DOWN / DEAD tag, drawn the heaviest in the blood family per the contract's "RED bold".
    const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
        }
    }

    /// The pop's combat-text string (consumed by reference at spawn — the caller clones the
    /// inner [`CombatText`] into the [`Text2d`](bevy::prelude::Text2d)).
    pub(in crate::fx) const fn text(&self) -> &CombatText {
        &self.text
    }

    /// The pop's valence swatch — the [`Color`](bevy::prelude::Color) fed straight to
    /// [`spawn_floating_text`](super::text::spawn_floating_text).
    pub(in crate::fx) const fn color(&self) -> Color {
        self.color
    }

    /// The pop's styling weight ([`FctEmphasis::Bold`] for the lethal tag, else
    /// [`FctEmphasis::Normal`]).
    pub(in crate::fx) const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }
}

/// The `(cell, level)` a round's pops anchor over.
///
/// For a [`ShotKind::Ganger`] outcome it reads the struck ganger's current
/// [`Position`](gdtf_battle_sim::Position) (so the pops sit on the body that was hit, even if
/// that body has since moved off the impact cell) — reconstructing the typed [`Cell`] /
/// [`Level`] from the position's `IVec3` (the readers.rs `cell_and_level` idiom). For any other
/// kind (cover / slab / ground / miss), or a ganger whose [`Position`] is missing (fail-closed),
/// it falls back to the round's impact `(cell, level)` — where the round landed.
///
/// `pub(in crate::fx)`: called by [`spawn_shot_projectiles`](super::super::spawn_shot_projectiles)
/// at projectile-spawn time so the anchor is captured AT THE SHOT and threaded through the
/// staggered projectile → impact pipeline (so the pop still sits on the body that was hit even
/// if it has moved by the time the staggered impact lands).
pub(in crate::fx) fn anchor_cell(msg: &ShotFired, positions: &Query<&Position>) -> (Cell, Level) {
    if let ShotKind::Ganger(entity) = msg.kind
        && let Ok(pos) = positions.get(entity)
    {
        let cell = Cell::new(pos.x, pos.y);
        let storey = u8::try_from(pos.z).unwrap_or(0);
        return (cell, Level::new(storey));
    }
    (msg.impact_cell, msg.impact_level)
}

/// Classify one round's [`HitReport`] into the ordered list of pops it yields.
///
/// The ordering is fixed so a multi-event round reads top-to-bottom in severity order: the HP
/// damage number first, then the wound tag, the penetration verdict, and finally the
/// DOWN / DEAD lethal tag (the heaviest pop, lowest in the stack so it reads last). A clean
/// miss — a non-connecting shot (or a `None` report) — yields NO pops at all. A graze (severity
/// `None` on a ganger hit) is a CONNECTING shot and still yields a GREY `"Grazed"` instead of a
/// wound tag.
///
/// Split out so the event-to-pop mapping is unit-testable without an [`App`] (the test feeds a
/// synthesized [`HitReport`] and asserts the exact pop list) AND reusable: it is the SHARED
/// classification [`spawn_shot_projectiles`](super::super::spawn_shot_projectiles) calls at
/// projectile-spawn time, so the SAME mapping that drove the immediate-spawn reader now rides
/// the staggered projectile → impact pipeline.
///
/// `pub(in crate::fx)`: called by the sibling `projectile` module; the classification stays
/// private to the FX layer.
pub(in crate::fx) fn classify_report(report: Option<&HitReport>) -> Vec<ClassifiedPop> {
    // A round with no report at all (a geometry-only message) is a clean miss — no pops.
    let Some(report) = report else {
        return Vec::new();
    };
    // A non-ganger outcome, or a ganger hit that applied nothing, did NOT connect — a clean
    // miss yields no floating text.
    let (ShotKind::Ganger(_), Some(applied)) = (report.kind, report.applied.as_ref()) else {
        return Vec::new();
    };

    let mut pops = Vec::new();

    // 1. HP damage number (RED) — only when the hit actually dealt HP loss.
    let hp_loss = *applied.hit.hp_damage;
    if hp_loss > 0 {
        pops.push(ClassifiedPop::new(
            CombatText::new(format!("-{hp_loss}")),
            valence_color(FctValence::Damage),
        ));
    }

    // 2. Wound gained (AMBER ramp) when a real wounding tier landed on a named part; else a
    //    graze (severity None) reads GREY "Grazed" (HP loss, no Wound spent — resolution.md §6).
    pops.push(wound_or_graze_pop(applied.severity, report.part));

    // 3. Penetration verdict — "Penetrated" (GREY, it went through) vs "Deflected" (AMBER, the
    //    armor soaked it). penetrating > 0 = the hit punched through.
    pops.push(penetration_pop(*applied.hit.penetrating));

    // 4. DOWN / DEAD (lethal RED, BOLD + uppercased) on a life-state transition.
    if let Some(lethal) = lethal_pop(applied.life_after) {
        pops.push(lethal);
    }

    pops
}

/// The wound / graze pop for a ganger hit's [`Severity`] + struck [`BodyPart`].
///
/// A real wounding tier ([`Minor`](Severity::Minor) → [`Fatal`](Severity::Fatal)) with a named
/// part reads `"<Part> <Tier>"` in the [`severity_color`] amber-to-lethal ramp. A
/// [`Severity::None`] graze (or a wounding tier with no part named, defensively) reads a GREY
/// `"Grazed"` — HP loss but no Wound spent.
fn wound_or_graze_pop(severity: Severity, part: Option<BodyPart>) -> ClassifiedPop {
    match (severity, part) {
        (Severity::None, _) | (_, None) => ClassifiedPop::new(
            CombatText::new("Grazed"),
            valence_color(FctValence::Neutral),
        ),
        (tier, Some(part)) => ClassifiedPop::new(
            CombatText::new(format!(
                "{} {}",
                body_part_label(part),
                severity_label(tier)
            )),
            severity_color(tier),
        ),
    }
}

/// The penetration-verdict pop for a hit's penetrating-damage magnitude.
///
/// `> 0` = the hit PENETRATED the armor — a neutral GREY `"Penetrated"` (the bad news for the
/// defender, but a status note, not a damage number). `<= 0` = the armor DEFLECTED / absorbed
/// it — an AMBER `"Deflected"` (the armor did its job). The damage number itself is a separate
/// pop; this one reports the armor verdict.
fn penetration_pop(penetrating: i32) -> ClassifiedPop {
    if penetrating > 0 {
        ClassifiedPop::new(
            CombatText::new("Penetrated"),
            valence_color(FctValence::Neutral),
        )
    } else {
        ClassifiedPop::new(
            CombatText::new("Deflected"),
            valence_color(FctValence::Wound),
        )
    }
}

/// The DOWN / DEAD lethal pop for a hit's resulting [`LifeState`], or [`None`] when the target
/// is still [`Alive`](LifeState::Alive).
///
/// A [`Downed`](LifeState::Downed) / [`Dead`](LifeState::Dead) transition reads the uppercased
/// `"DOWN"` / `"DEAD"` in the lethal RED ([`FctValence::Lethal`]) drawn [`FctEmphasis::Bold`]
/// — the heaviest pop in the blood family, delivering the contract's "DOWN / DEAD (RED bold)".
/// The all-caps tag is a complementary cue alongside the bold weight + larger size, not a
/// substitute for it.
fn lethal_pop(life_after: LifeState) -> Option<ClassifiedPop> {
    let tag = match life_after {
        LifeState::Alive => return None,
        LifeState::Downed => "DOWN",
        LifeState::Dead => "DEAD",
    };
    Some(ClassifiedPop::new_bold(
        CombatText::new(tag),
        valence_color(FctValence::Lethal),
    ))
}

/// The human-readable label for a struck [`BodyPart`] — the L/R limbs collapse to the
/// ticket's `"Arm"` / `"Leg"` (the pop names the part, not the side).
const fn body_part_label(part: BodyPart) -> &'static str {
    match part {
        BodyPart::Head => "Head",
        BodyPart::Torso => "Torso",
        BodyPart::LeftArm | BodyPart::RightArm => "Arm",
        BodyPart::LeftLeg | BodyPart::RightLeg => "Leg",
    }
}

/// The human-readable label for a wounding [`Severity`] tier.
///
/// Only the wounding tiers ([`Minor`](Severity::Minor) → [`Fatal`](Severity::Fatal)) reach
/// here ([`wound_or_graze_pop`] handles [`None`](Severity::None) as a graze before calling
/// this); [`None`](Severity::None) maps to `"Graze"` as a defensive fallback so the function is
/// total.
const fn severity_label(tier: Severity) -> &'static str {
    match tier {
        Severity::None => "Graze",
        Severity::Minor => "Minor",
        Severity::Major => "Major",
        Severity::Critical => "Critical",
        Severity::Fatal => "Fatal",
    }
}

#[cfg(test)]
mod test {
    use bevy::ecs::entity::Entity;
    use gdtf_battle_sim::{
        AppliedDamage, BodyPart, HitReport, HitResult, HpDamage, IntegrityWear, LifeState, Matchup,
        PenetratingDamage, Severity, ShotKind,
    };

    use super::{
        ClassifiedPop, FctEmphasis, FctValence, classify_report, severity_color, valence_color,
    };

    /// A `HitResult` carrying `hp` HP loss + `pen` penetrating damage (zero wear — the wear
    /// number is not an FCT input this slice).
    fn hit_result(hp: i32, pen: i32) -> HitResult {
        HitResult {
            penetrating: PenetratingDamage::new(pen),
            hp_damage:   HpDamage::new(hp),
            wear:        IntegrityWear::new(0),
        }
    }

    /// A ganger-hit `HitReport` for `part` with `hp` HP loss / `pen` penetration / `severity`
    /// tier / `life_after` state — the synthesized report the classifier reads.
    fn ganger_report(
        part: BodyPart,
        hp: i32,
        pen: i32,
        severity: Severity,
        life_after: LifeState,
    ) -> HitReport {
        HitReport {
            // The classifier only matches on the Ganger variant — it never dereferences the
            // entity — so a placeholder handle is enough to drive the Ganger branch.
            kind:    ShotKind::Ganger(Entity::PLACEHOLDER),
            part:    Some(part),
            applied: Some(AppliedDamage {
                matchup: Matchup::Neutral,
                hit: hit_result(hp, pen),
                severity,
                life_after,
                broken: None,
                worn: None,
            }),
        }
    }

    /// The set of `(text, color)` pairs a report classifies into, for order-insensitive
    /// membership asserts.
    fn pop_pairs(report: Option<&HitReport>) -> Vec<(String, bevy::prelude::Color)> {
        classify_report(report)
            .into_iter()
            .map(|ClassifiedPop { text, color, .. }| ((*text).clone(), color))
            .collect()
    }

    /// Whether the classified pops contain a pop with exactly `text` in exactly `color`.
    fn has_pop(report: Option<&HitReport>, text: &str, color: bevy::prelude::Color) -> bool {
        pop_pairs(report)
            .iter()
            .any(|(t, c)| t == text && *c == color)
    }

    /// The [`FctEmphasis`] of the classified pop with exactly `text`, or `None` if absent.
    fn pop_emphasis(report: Option<&HitReport>, text: &str) -> Option<FctEmphasis> {
        classify_report(report)
            .into_iter()
            .find(|pop| *pop.text == *text)
            .map(|pop| pop.emphasis)
    }

    /// A damaging ganger hit yields the RED HP-loss number `-N`.
    #[test]
    fn a_damage_hit_yields_the_red_hp_number() {
        let report = ganger_report(BodyPart::Torso, 7, 5, Severity::Minor, LifeState::Alive);
        assert!(
            has_pop(Some(&report), "-7", valence_color(FctValence::Damage)),
            "a 7-HP hit must yield a RED \"-7\" pop, got {:?}",
            pop_pairs(Some(&report)),
        );
    }

    /// A wounding hit yields the `"<Part> <Tier>"` pop in the severity AMBER ramp (and the L/R
    /// limb collapses to the side-less label).
    #[test]
    fn a_wound_yields_the_part_tier_pop_in_the_severity_color() {
        let report = ganger_report(BodyPart::LeftArm, 4, 3, Severity::Major, LifeState::Alive);
        assert!(
            has_pop(Some(&report), "Arm Major", severity_color(Severity::Major)),
            "a Major left-arm wound must yield \"Arm Major\" in the Major amber, got {:?}",
            pop_pairs(Some(&report)),
        );
    }

    /// A graze (severity `None`) yields the GREY `"Grazed"` pop (HP loss, no Wound), NOT a wound
    /// tag.
    #[test]
    fn a_graze_yields_grey_grazed_not_a_wound() {
        let report = ganger_report(BodyPart::Torso, 2, 0, Severity::None, LifeState::Alive);
        let pairs = pop_pairs(Some(&report));
        assert!(
            has_pop(Some(&report), "Grazed", valence_color(FctValence::Neutral)),
            "a Severity::None hit must yield a GREY \"Grazed\" pop, got {pairs:?}",
        );
        assert!(
            !pairs.iter().any(|(t, _)| t.contains("Torso")),
            "a graze must NOT yield a body-part wound tag, got {pairs:?}",
        );
    }

    /// A clean miss — a non-connecting shot (or a `None` report) — yields NO pops at all: a
    /// missed shot spawns no floating combat text.
    #[test]
    fn a_clean_miss_yields_no_pops() {
        let miss = HitReport::no_effect(ShotKind::Miss);
        let pairs = pop_pairs(Some(&miss));
        assert!(
            pairs.is_empty(),
            "a clean miss must yield no pops, got {pairs:?}"
        );
        // A round with NO report at all (geometry-only message) is likewise a clean miss.
        let none_pairs = pop_pairs(None);
        assert!(
            none_pairs.is_empty(),
            "a None report must yield no pops, got {none_pairs:?}"
        );
    }

    /// A hit that DOWNS the target yields the lethal-RED `"DOWN"` pop; a hit that KILLS yields
    /// `"DEAD"`. Both are drawn [`FctEmphasis::Bold`] (the contract's "RED bold"), distinct from
    /// the ordinary body-weight HP-damage pop. An Alive outcome yields neither.
    #[test]
    fn a_down_and_a_dead_yield_the_lethal_red_bold_tag() {
        let down = ganger_report(BodyPart::Torso, 9, 6, Severity::Critical, LifeState::Downed);
        assert!(
            has_pop(Some(&down), "DOWN", valence_color(FctValence::Lethal)),
            "a Downed outcome must yield a lethal-RED \"DOWN\" pop, got {:?}",
            pop_pairs(Some(&down)),
        );
        assert_eq!(
            pop_emphasis(Some(&down), "DOWN"),
            Some(FctEmphasis::Bold),
            "the \"DOWN\" pop must be BOLD (the contract's \"RED bold\"), not body weight",
        );
        // The same round's ordinary HP-damage pop stays body weight — bold is reserved for the
        // lethal tag, so the descope to all-caps-only is no longer the emphasis.
        assert_eq!(
            pop_emphasis(Some(&down), "-9"),
            Some(FctEmphasis::Normal),
            "an ordinary HP-damage pop must stay body weight, not bold",
        );

        let dead = ganger_report(BodyPart::Head, 12, 10, Severity::Fatal, LifeState::Dead);
        assert!(
            has_pop(Some(&dead), "DEAD", valence_color(FctValence::Lethal)),
            "a Dead outcome must yield a lethal-RED \"DEAD\" pop, got {:?}",
            pop_pairs(Some(&dead)),
        );
        assert_eq!(
            pop_emphasis(Some(&dead), "DEAD"),
            Some(FctEmphasis::Bold),
            "the \"DEAD\" pop must be BOLD (the contract's \"RED bold\"), not body weight",
        );

        // An Alive outcome yields no DOWN/DEAD tag.
        let alive = ganger_report(BodyPart::Torso, 3, 2, Severity::Minor, LifeState::Alive);
        let alive_pairs = pop_pairs(Some(&alive));
        assert!(
            !alive_pairs.iter().any(|(t, _)| t == "DOWN" || t == "DEAD"),
            "an Alive outcome must yield no DOWN/DEAD tag, got {alive_pairs:?}",
        );
    }

    /// The penetration verdict pin-discriminates: a penetrating hit (`pen > 0`) yields the GREY
    /// `"Penetrated"` pop; a deflected hit (`pen <= 0`) yields the AMBER `"Deflected"` pop.
    #[test]
    fn the_penetration_verdict_discriminates_penetrated_from_deflected() {
        let through = ganger_report(BodyPart::Torso, 5, 4, Severity::Minor, LifeState::Alive);
        assert!(
            has_pop(
                Some(&through),
                "Penetrated",
                valence_color(FctValence::Neutral)
            ),
            "pen > 0 must yield a GREY \"Penetrated\" pop, got {:?}",
            pop_pairs(Some(&through)),
        );

        let soaked = ganger_report(BodyPart::Torso, 1, 0, Severity::None, LifeState::Alive);
        assert!(
            has_pop(Some(&soaked), "Deflected", valence_color(FctValence::Wound)),
            "pen == 0 must yield an AMBER \"Deflected\" pop, got {:?}",
            pop_pairs(Some(&soaked)),
        );
    }
}

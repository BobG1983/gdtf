//! The [`HitReport`] → pops classification — the flesh + structural families and
//! their labels.

use gdtf_battle_sim::{
    armor::BodyPart,
    prelude::{CellLevel, LifeState},
    resolve_and_apply::{GangerVerdict, HitReport, HitVerdict},
    severity::Severity,
};

use super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        text::CombatText,
    },
    classified::ClassifiedPop,
};

/// Classify one round's [`HitReport`] into the ordered list of pops it yields.
///
/// The presenter's ONE exhaustive view-side dispatch over the report's per-kind
/// [`HitVerdict`] (GTW-573 C5 — a new struck kind is a compile error here until it is
/// given a pop family): a ganger verdict yields the FLESH family (HP number, wound /
/// graze tag, penetration verdict, DOWN / DEAD tag — in that fixed severity order, the
/// lethal tag lowest so it reads last), a cover / slab / ground verdict yields the
/// STRUCTURAL family (a `"<Noun> hit"` chip indicator, a bold `"<Noun> Destroyed"` tag,
/// or a cosmetic `"Dust"` cue — GTW-386), and ONLY a no-effect verdict — a clean miss,
/// a corpse-skip (or a `None` report) — yields NO pops at all. A graze (severity `None`
/// on a ganger verdict) is a CONNECTING shot and still yields a GREY `"Grazed"` instead
/// of a wound tag.
///
/// Split out so the event-to-pop mapping is unit-testable without an
/// [`App`](bevy::prelude::App) (the test feeds a synthesized [`HitReport`] and asserts the
/// exact pop list) AND reusable: it is the SHARED classification
/// [`spawn_shot_projectiles`](super::super::super::spawn_shot_projectiles) calls at
/// projectile-spawn time, so the SAME mapping that drove the immediate-spawn reader now rides
/// the staggered projectile → impact pipeline.
///
/// `pub(in crate::actors::fx)`: called by the sibling `projectile` module; the classification stays
/// private to the FX layer.
pub(in crate::actors::fx) fn classify_report(report: Option<&HitReport>) -> Vec<ClassifiedPop> {
    // A round with no report at all (a geometry-only message) is a clean miss — no pops.
    let Some(report) = report else {
        return Vec::new();
    };
    // Each VERDICT yields its own family of pops. A ganger verdict reads the flesh family
    // (HP / wound / penetration / DOWN-DEAD); a structural verdict (cover / slab / ground)
    // reads the STRUCTURAL family — a "hit" / "Destroyed" / "Dust" indicator, NOT flesh — so
    // a shot that struck the world still pops feedback instead of falling through to a
    // phantom miss (GTW-386). Only a no-effect verdict (a genuine clean miss / corpse-skip)
    // yields NO pops.
    match &report.verdict {
        HitVerdict::Ganger(verdict) => ganger_pops(verdict),
        HitVerdict::Cover(cover) => structural_pops(StructuralKind::Cover, cover.destroyed),
        HitVerdict::Slab(slab) => structural_pops(StructuralKind::Slab, slab.destroyed),
        HitVerdict::Ground(_) => ground_pops(),
        HitVerdict::NoEffect => Vec::new(),
    }
}

/// The flesh-family pops for a landed ganger hit's [`GangerVerdict`] — the HP-loss
/// number, the wound / graze tag, the penetration verdict, and the DOWN / DEAD lethal
/// tag, in that fixed order.
///
/// A ganger verdict ALWAYS means the hit connected (GTW-573: a corpse-skip / no-part
/// round folds to [`HitVerdict::NoEffect`], which [`classify_report`] already dropped),
/// so there is no not-connected branch here. Split out of [`classify_report`] so the
/// per-kind dispatch reads as one branch each.
fn ganger_pops(verdict: &GangerVerdict) -> Vec<ClassifiedPop> {
    let applied = &verdict.applied;

    let mut pops = Vec::new();

    // 1. HP damage number (RED) — only when the hit actually dealt HP loss.
    let hp_loss = *applied.hit.hp_damage;
    if hp_loss > 0 {
        pops.push(ClassifiedPop::new(
            CombatText::new(format!("-{hp_loss}")),
            valence_color(FctValence::Damage),
        ));
    }

    // 2. Wound gained (AMBER ramp) when a real wounding tier landed; else a graze
    //    (severity None) reads GREY "Grazed" (HP loss, no Wound spent — resolution.md §6).
    pops.push(wound_or_graze_pop(applied.severity, verdict.part));

    // 3. Penetration verdict — "Armor pierced" (GREY, it went through) vs "Armor held" (AMBER,
    //    the armor soaked it). penetrating > 0 = the hit punched through.
    pops.push(penetration_pop(*applied.hit.penetrating));

    // 4. DOWN / DEAD (lethal RED, BOLD + uppercased) on a life-state transition.
    if let Some(lethal) = lethal_pop(applied.life_after) {
        pops.push(lethal);
    }

    pops
}

/// Which destructible structure a structural hit struck — the noun the structural pop names
/// (`"Cover"` / `"Slab"`).
///
/// A small presenter-side label enum (no bare string): [`structural_pops`] reads it for both
/// the structural-hit indicator (`"<Noun> hit"`) and the destruction tag (`"<Noun> Destroyed"`),
/// so the two share one spelling of the noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralKind {
    /// A wall / prop piece of cover (a [`ShotKind::Cover`](gdtf_battle_sim::resolve_coarse::ShotKind::Cover)
    /// outcome).
    Cover,
    /// A floor / roof slab (a [`ShotKind::Slab`](gdtf_battle_sim::resolve_coarse::ShotKind::Slab) outcome).
    Slab,
}

impl StructuralKind {
    /// The display noun this structure pops as — `"Cover"` / `"Slab"`.
    const fn noun(self) -> &'static str {
        match self {
            Self::Cover => "Cover",
            Self::Slab => "Slab",
        }
    }
}

/// The structural-family pop(s) for a cover / slab hit (GTW-386).
///
/// The [`HitReport`] carries NO per-hit cover/slab DAMAGE NUMBER (its `applied` block is `None` for
/// a structural hit — the precise HP removed is computed inside the sim's deplete step and not
/// surfaced on the report; see C5 / the report follow-up note), so this shows the QUALITATIVE
/// structural feedback the report DOES carry: the destruction verdict. When `destroyed` is
/// `Some` (this round depleted the structure's HP to zero) it reads the emphatic `"<Noun>
/// Destroyed"` in the lethal RED, BOLD — the structural mirror of a ganger's DOWN / DEAD tag
/// (consistent heaviest-pop visual language, but the word reads STRUCTURAL, not flesh).
/// Otherwise it reads a neutral GREY `"<Noun> hit"` — a qualitative chip indicator that the
/// round struck (and chipped) the structure, distinct from the flesh-red HP number a ganger
/// hit leads with. Always at least one pop, so a structural hit never falls through to a
/// phantom miss.
fn structural_pops(kind: StructuralKind, destroyed: Option<CellLevel>) -> Vec<ClassifiedPop> {
    let noun = kind.noun();
    if destroyed.is_some() {
        vec![ClassifiedPop::new_bold(
            CombatText::new(format!("{noun} Destroyed")),
            valence_color(FctValence::Lethal),
        )]
    } else {
        vec![ClassifiedPop::new(
            CombatText::new(format!("{noun} hit")),
            valence_color(FctValence::Neutral),
        )]
    }
}

/// The structural-family pop for a [`ShotKind::Ground`](gdtf_battle_sim::resolve_coarse::ShotKind::Ground)
/// hit (GTW-386).
///
/// The ground is **damaged, never destroyed** — its accrual is purely cosmetic
/// (`docs/combat/resolution.md` §3.2). So rather than a damage number this reads a single
/// minor neutral-GREY `"Dust"` impact indicator (a kicked-up-dust cue at the ground impact),
/// keeping the structural visual language without implying flesh damage or destruction.
fn ground_pops() -> Vec<ClassifiedPop> {
    vec![ClassifiedPop::new(
        CombatText::new("Dust"),
        valence_color(FctValence::Neutral),
    )]
}

/// The wound / graze pop for a ganger hit's [`Severity`] + struck [`BodyPart`].
///
/// A real wounding tier ([`Minor`](Severity::Minor) → [`Fatal`](Severity::Fatal)) reads
/// `"<Part> <Tier>"` in the [`severity_color`] amber-to-lethal ramp. A
/// [`Severity::None`] graze reads a GREY `"Grazed"` — HP loss but no Wound spent. (The
/// old part-less defensive arm is gone: a [`GangerVerdict`] always names its struck
/// part — GTW-573 made the part-less applied state unrepresentable.)
fn wound_or_graze_pop(severity: Severity, part: BodyPart) -> ClassifiedPop {
    match severity {
        Severity::None => ClassifiedPop::new(
            CombatText::new("Grazed"),
            valence_color(FctValence::Neutral),
        ),
        tier => ClassifiedPop::new(
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
/// `> 0` = the hit punched through the armor — a neutral GREY `"Armor pierced"` (the bad news
/// for the defender, but a status note, not a damage number). `<= 0` = the armor held — an
/// AMBER `"Armor held"` (the armor did its job). The damage number itself is a separate pop;
/// this one reports the armor verdict.
fn penetration_pop(penetrating: i32) -> ClassifiedPop {
    if penetrating > 0 {
        ClassifiedPop::new(
            CombatText::new("Armor pierced"),
            valence_color(FctValence::Neutral),
        )
    } else {
        ClassifiedPop::new(
            CombatText::new("Armor held"),
            valence_color(FctValence::Status),
        )
    }
}

/// The DOWN / DEAD lethal pop for a hit's resulting [`LifeState`], or [`None`] when the target
/// is still [`Alive`](LifeState::Alive).
///
/// A [`Downed`](LifeState::Downed) / [`Dead`](LifeState::Dead) transition reads the uppercased
/// `"DOWN"` / `"DEAD"` in the lethal RED ([`FctValence::Lethal`]) drawn
/// [`FctEmphasis::Bold`](super::super::text::FctEmphasis::Bold)
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

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

pub(in crate::actors::fx) fn classify_report(report: Option<&HitReport>) -> Vec<ClassifiedPop> {
    let Some(report) = report else {
        return Vec::new();
    };
    match &report.verdict {
        HitVerdict::Ganger(verdict) => ganger_pops(verdict),
        HitVerdict::Cover(cover) => structural_pops(StructuralKind::Cover, cover.destroyed),
        HitVerdict::Slab(slab) => structural_pops(StructuralKind::Slab, slab.destroyed),
        HitVerdict::Ground(_) => ground_pops(),
        HitVerdict::NoEffect => Vec::new(),
    }
}

fn ganger_pops(verdict: &GangerVerdict) -> Vec<ClassifiedPop> {
    let applied = &verdict.applied;

    let mut pops = Vec::new();

    let hp_loss = *applied.hit.hp_damage;
    if hp_loss > 0 {
        pops.push(ClassifiedPop::new(
            CombatText::new(format!("-{hp_loss}")),
            valence_color(FctValence::Damage),
        ));
    }

    pops.push(wound_or_graze_pop(applied.severity, verdict.part));

    pops.push(penetration_pop(*applied.hit.penetrating));

    if let Some(lethal) = lethal_pop(applied.life_after) {
        pops.push(lethal);
    }

    pops
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralKind {
            Cover,
        Slab,
}

impl StructuralKind {
        const fn noun(self) -> &'static str {
        match self {
            Self::Cover => "Cover",
            Self::Slab => "Slab",
        }
    }
}

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

fn ground_pops() -> Vec<ClassifiedPop> {
    vec![ClassifiedPop::new(
        CombatText::new("Dust"),
        valence_color(FctValence::Neutral),
    )]
}

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

const fn body_part_label(part: BodyPart) -> &'static str {
    match part {
        BodyPart::Head => "Head",
        BodyPart::Torso => "Torso",
        BodyPart::LeftArm | BodyPart::RightArm => "Arm",
        BodyPart::LeftLeg | BodyPart::RightLeg => "Leg",
    }
}

const fn severity_label(tier: Severity) -> &'static str {
    match tier {
        Severity::None => "Graze",
        Severity::Minor => "Minor",
        Severity::Major => "Major",
        Severity::Critical => "Critical",
        Severity::Fatal => "Fatal",
    }
}

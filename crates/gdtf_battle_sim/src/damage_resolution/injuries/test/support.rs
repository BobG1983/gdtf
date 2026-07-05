//! Shared fixtures for the injury tests — the minimal [`GainedInjury`] builders,
//! the fixed roll seed + rng, and the single-entry registry/table builder each
//! concern file reaches via `use super::support::*;`.

use super::super::{
    GainedInjury, InjuryDef, InjuryEffect, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
    InspectText, LogText, PopupText, PostHeal, StatDelta, StatTarget, WeightedInjuryEntry,
    WeightedInjuryTable,
};
use crate::{
    armor::BodyPart,
    rng::{BattleSeed, InjuryRng},
    severity::Severity,
};

/// Build a minimal [`GainedInjury`] carrying the given effects (the texts and the
/// part/severity are irrelevant to the folding under test).
pub(super) fn gained_with(effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        BodyPart::Head,
        Severity::Minor,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

/// Build a [`GainedInjury`] on a chosen `part` carrying the given effects — the
/// part-parameterized fixture the GTW-443 hand-count fold tests need (the disabled side
/// is derived from `part`).
pub(super) fn gained_on(part: BodyPart, effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        part,
        Severity::Major,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

/// A fixed seed for the roll tests (arbitrary, not tuned).
pub(super) const ROLL_SEED: u64 = 0x90A1_C401;

/// A fresh [`InjuryRng`] from the shared roll seed.
pub(super) fn injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(ROLL_SEED))
}

/// Build a single-entry registry + table for `(part, severity)` keyed by `key`, so a
/// roll for that bucket resolves a known [`InjuryDef`]. The def carries one `Modify`
/// effect (the magnitude is irrelevant — the tests assert STRUCTURE, not tuning).
pub(super) fn one_injury_table(
    key: &str,
    part: BodyPart,
    severity: Severity,
) -> (InjuryRegistry, InjuryTables) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name: name.clone(),
        category: part.injury_category(),
        severity,
        popup_text: PopupText::new("HURT".to_owned()),
        log_text: LogText::new("is hurt".to_owned()),
        inspect_text: InspectText::new("Hurt".to_owned()),
        effects: vec![InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-2),
        }],
        post_heal: PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    tables.insert(
        part.injury_category(),
        severity,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );
    (registry, tables)
}

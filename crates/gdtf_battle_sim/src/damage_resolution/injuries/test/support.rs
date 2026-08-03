use super::super::{
    DamageContext, GainedInjury, InjuryDef, InjuryEffect, InjuryName, InjuryRegistry, InjuryTables,
    InjuryWeight, InspectText, LogText, PopupText, PostHeal, StatDelta, StatTarget,
    WeightedInjuryEntry, WeightedInjuryTable,
};
use crate::{
    armor::BodyPart,
    rng::{BattleSeed, InjuryRng},
    severity::Severity,
};

pub(super) fn gained_with(effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        BodyPart::Head,
        Severity::Minor,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

pub(super) fn gained_on(part: BodyPart, effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        part,
        Severity::Major,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

pub(super) const ROLL_SEED: u64 = 0x90A1_C401;

pub(super) fn injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(ROLL_SEED))
}

pub(super) fn injury_rng_from(seed: u64) -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(seed))
}

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
        DamageContext::Ranged,
        severity,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );
    (registry, tables)
}

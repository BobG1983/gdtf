use super::super::{InjuryDef, InjuryEffect, PostHeal, StatTarget};
use crate::{armor::InjuryCategory, severity::Severity};

#[test]
fn injury_def_deserializes_from_the_schema_ron() {
    let ron = r#"(
        name:         "Lost Eye",
        category:     Head,
        severity:     Critical,
        popup_text:   "LOST EYE",
        log_text:     "loses an eye",
        inspect_text: "Lost Eye -- -2 Aim, -1 Cool",
        effects: [
            Modify(stat: Aim,  amount: -2),
            Modify(stat: Cool, amount: -1),
        ],
        post_heal:    Deferred,
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "schema RON must deserialize into InjuryDef: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };

    assert_eq!(def.category, InjuryCategory::Head);
    assert_eq!(def.severity, Severity::Critical);
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert_eq!(def.effects.len(), 2);
    assert!(matches!(
        def.effects[0],
        InjuryEffect::Modify {
            stat: StatTarget::Aim,
            ..
        }
    ));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Cool,
            ..
        }
    ));
}

#[test]
fn injury_def_post_heal_defaults_to_deferred_when_omitted() {
    let ron = r#"(
        name:         "Scalp Graze",
        category:     Head,
        severity:     Minor,
        popup_text:   "SCALP GRAZE",
        log_text:     "is grazed across the scalp",
        inspect_text: "Scalp Graze",
        effects: [ Bleeding(amount: 1) ],
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "RON without post_heal must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert!(matches!(def.effects[0], InjuryEffect::Bleeding { .. }));
}

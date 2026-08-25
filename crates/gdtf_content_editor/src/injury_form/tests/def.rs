use gdtf_assets::serialize_ron_pretty;
use gdtf_battle_sim::injuries::{InjuryDef, InjuryEffect, InjuryName, StatDelta, StatTarget};

use crate::injury_form::{draft::InjuryDraft, save::draft_to_def};

fn fixture_def() -> InjuryDef {
    let ron = "(name: \"Test Wound\", category: Arm, severity: Major, \
               popup_text: \"WOUNDED\", log_text: \"is wounded\", \
               inspect_text: \"Test Wound -- fixture\", \
               effects: [Modify(stat: Aim, amount: -2), Bleeding(amount: 1), \
                         DisableHand, MovementCostMul(1.5)])";
    let parsed = ron::de::from_str::<InjuryDef>(ron);
    assert!(parsed.is_ok(), "fixture def must parse: {parsed:?}");
    let Ok(def) = parsed else {
        unreachable!("asserted Ok above")
    };
    def
}

#[test]
fn default_is_pristine_and_load_injury_fills_the_form() {
    let mut draft = InjuryDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );
    assert_eq!(draft.key(), "");
    assert_eq!(
        draft.def().effects.len(),
        1,
        "a pristine def carries one seed effect row (the schema's `effects ≥ 1`)",
    );

    let def = fixture_def();
    draft.load_injury(&InjuryName::new("test_wound".to_owned()), &def);
    assert!(
        !draft.autoload_pending(),
        "a loaded injury ends the autoload"
    );
    assert_eq!(draft.key(), "test_wound");
    assert_eq!(draft.def(), &def);

    let minted = InjuryDraft::new_injury();
    assert!(
        !minted.autoload_pending(),
        "a deliberate new injury never re-seeds"
    );
    assert_eq!(minted.key(), "");

    let mut pristine = InjuryDraft::default();
    pristine.mark_autoloaded();
    assert!(
        !pristine.autoload_pending(),
        "the empty-registry branch ends the seed"
    );
}

#[test]
fn edited_def_round_trips_through_the_loader_schema() {
    let mut edited = InjuryDraft::new_injury();
    edited.load_injury(&InjuryName::new("test_wound".to_owned()), &fixture_def());
    edited.def_mut().effects.push(InjuryEffect::Modify {
        stat:   StatTarget::Grit,
        amount: StatDelta::new(3),
    });

    let (key, def) = draft_to_def(&edited);
    let serialized = serialize_ron_pretty(&def);
    assert!(
        serialized.is_ok(),
        "serializing the edited def must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded_def = ron::de::from_str::<InjuryDef>(&serialized);
    assert!(
        reloaded_def.is_ok(),
        "the serialized def must round-trip through the InjuryDef deserializer: {:?}",
        reloaded_def.as_ref().err(),
    );
    let Ok(reloaded_def) = reloaded_def else {
        return;
    };

    let mut reloaded = InjuryDraft::new_injury();
    reloaded.load_injury(&key, &reloaded_def);
    assert_eq!(
        reloaded, edited,
        "the reloaded injury must equal the edited draft (key + every def field incl. \
         the five effect rows) — the stem-key round-trip",
    );
}

#[test]
fn the_last_injury_effect_cannot_be_removed() {
    let mut draft = InjuryDraft::new_injury();
    assert_eq!(draft.effects().len(), 1, "a fresh draft seeds one effect");

    assert!(
        !draft.remove_effect(0),
        "the only effect reports nothing removed",
    );
    assert_eq!(
        draft.effects().len(),
        1,
        "the refused removal leaves the list untouched",
    );

    draft.add_effect();
    assert!(
        draft.remove_effect(0),
        "with two effects authored, one may go",
    );
    assert_eq!(
        draft.effects().len(),
        1,
        "the removal stops at the structural minimum",
    );

    draft.add_effect();
    assert!(
        !draft.set_effect(2, InjuryEffect::DisableHand),
        "index 2 is past the end of a two-effect list",
    );
    assert!(
        draft.set_effect(0, InjuryEffect::DisableHand),
        "index 0 is in bounds",
    );
    assert!(
        matches!(draft.effects()[0], InjuryEffect::DisableHand),
        "the in-bounds write landed on the first row",
    );
}

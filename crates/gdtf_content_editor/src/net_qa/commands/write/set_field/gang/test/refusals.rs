use gdtf_battle_sim::{
    armor::ArmorRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

use super::{
    super::write,
    fixtures::{FIRST, SEEDED_KEY, armor, asked_key, melee_weapons, one_member, refusal, weapons},
};
use crate::{
    gang_form::GangDraft,
    net_qa::wire::{
        EditorDraftNameNet, EditorListIndexNet, GangAttributeNet, GangAttributeValueNet,
        GangFieldNet,
    },
};

// Which refusal a write answered and the line it carries.
fn refused(
    draft: &mut GangDraft,
    weapons: Option<&WeaponRegistry>,
    melee: Option<&MeleeWeaponRegistry>,
    armor: Option<&ArmorRegistry>,
    field: GangFieldNet,
) -> (&'static str, String) {
    match refusal(write(draft, weapons, melee, armor, field)) {
        Ok(parts) => parts,
        Err(wrong) => unreachable!("{wrong}"),
    }
}

#[test]
fn an_attribute_over_the_range_the_drag_offers_is_refused_bad_arguments() {
    let mut draft = one_member();
    let over = GangAttributeValueNet::new(GangDraft::ATTRIBUTE_RANGE.end() + 1.0);

    let (kind, line) = refused(
        &mut draft,
        Some(&weapons()),
        Some(&melee_weapons()),
        Some(&armor()),
        GangFieldNet::MemberAttribute {
            index:     FIRST,
            attribute: GangAttributeNet::Speed,
            value:     over,
        },
    );

    assert_eq!(kind, "BadArguments", "got `{line}`");
    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert!(
        (*member.speed - 0.0).abs() < f32::EPSILON,
        "a writer that clamped instead of refusing would have stored a value here",
    );
}

#[test]
fn an_attribute_under_the_range_the_drag_offers_is_refused_bad_arguments() {
    let mut draft = one_member();
    let under = GangAttributeValueNet::new(GangDraft::ATTRIBUTE_RANGE.start() - 1.0);

    let (kind, _) = refused(
        &mut draft,
        Some(&weapons()),
        Some(&melee_weapons()),
        Some(&armor()),
        GangFieldNet::MemberAttribute {
            index:     FIRST,
            attribute: GangAttributeNet::Luck,
            value:     under,
        },
    );

    assert_eq!(kind, "BadArguments");
}

#[test]
fn a_member_index_past_the_end_of_the_roster_is_refused_bad_arguments() {
    let mut draft = one_member();
    let past = EditorListIndexNet::new(draft.members().len());

    let (kind, line) = refused(
        &mut draft,
        Some(&weapons()),
        Some(&melee_weapons()),
        Some(&armor()),
        GangFieldNet::MemberName {
            index: past,
            name:  EditorDraftNameNet::new("Kez"),
        },
    );

    assert_eq!(kind, "BadArguments");
    assert!(line.contains("past the end"), "got `{line}`");
    assert_eq!(
        draft.members().len(),
        1,
        "the refused write added no member"
    );
}

#[test]
fn a_key_none_of_the_three_registries_holds_is_refused_bad_arguments_naming_it() {
    let mut draft = one_member();
    for field in [
        GangFieldNet::MemberWeapon {
            index: FIRST,
            key:   Some(asked_key("no_such_key")),
        },
        GangFieldNet::MemberArmor {
            index: FIRST,
            key:   Some(asked_key("no_such_key")),
        },
        GangFieldNet::MemberMeleeWeapon {
            index: FIRST,
            key:   Some(asked_key("no_such_key")),
        },
    ] {
        let (kind, line) = refused(
            &mut draft,
            Some(&weapons()),
            Some(&melee_weapons()),
            Some(&armor()),
            field,
        );
        assert_eq!(kind, "BadArguments", "got `{line}`");
        assert!(
            line.contains("no_such_key"),
            "the line names the key: `{line}`"
        );
    }

    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert_eq!(member.weapon, None);
    assert_eq!(member.armor, None);
    assert_eq!(member.melee_weapon, None);
}

#[test]
fn a_registry_that_is_absent_answers_missing_model() {
    let mut draft = one_member();

    let (kind, _) = refused(
        &mut draft,
        None,
        Some(&melee_weapons()),
        Some(&armor()),
        GangFieldNet::MemberWeapon {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );

    assert_eq!(kind, "MissingModel");
}

#[test]
fn a_registry_that_is_present_and_empty_answers_missing_model() {
    let mut draft = one_member();
    let empty = ArmorRegistry::new([]);

    let (kind, _) = refused(
        &mut draft,
        Some(&weapons()),
        Some(&melee_weapons()),
        Some(&empty),
        GangFieldNet::MemberArmor {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );

    assert_eq!(
        kind, "MissingModel",
        "an empty registry offers the combo no row, which is a missing model rather than a bad \
         argument",
    );
}

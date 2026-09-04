use super::{
    super::write,
    fixtures::{FIRST, SEEDED_KEY, answered, armor, asked_key, melee_weapons, one_member, weapons},
};
use crate::mcp::wire::{
    EditorDraftNameNet, EditorKeyNet, GangAttributeNet, GangAttributeValueNet, GangFieldNet,
};

// The value each attribute is written with, so no two arms can share one.
fn value_for(attribute: GangAttributeNet) -> f32 {
    match attribute {
        GangAttributeNet::Speed => 11.0,
        GangAttributeNet::Aim => 22.0,
        GangAttributeNet::Strength => 33.0,
        GangAttributeNet::Toughness => 44.0,
        GangAttributeNet::Reflexes => 55.0,
        GangAttributeNet::Cool => 66.0,
        GangAttributeNet::Grit => 77.0,
        GangAttributeNet::Luck => 88.0,
    }
}

// One Gang write with every registry the form reads in the world.
fn write_field(
    draft: &mut crate::gang_form::GangDraft,
    field: GangFieldNet,
) -> Result<GangFieldNet, String> {
    answered(write(
        draft,
        Some(&weapons()),
        Some(&melee_weapons()),
        Some(&armor()),
        field,
    ))
}

#[test]
fn each_attribute_arm_writes_its_own_field_of_the_member() {
    let mut draft = one_member();
    for attribute in GangAttributeNet::ALL {
        let written = write_field(
            &mut draft,
            GangFieldNet::MemberAttribute {
                index: FIRST,
                attribute,
                value: GangAttributeValueNet::new(value_for(attribute)),
            },
        );
        assert!(
            written.is_ok(),
            "an attribute inside the range the drag offers is written: {written:?}",
        );
    }

    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    for (attribute, held) in [
        (GangAttributeNet::Speed, *member.speed),
        (GangAttributeNet::Aim, *member.aim),
        (GangAttributeNet::Strength, *member.strength),
        (GangAttributeNet::Toughness, *member.toughness),
        (GangAttributeNet::Reflexes, *member.reflexes),
        (GangAttributeNet::Cool, *member.cool),
        (GangAttributeNet::Grit, *member.grit),
        (GangAttributeNet::Luck, *member.luck),
    ] {
        assert!(
            (held - value_for(attribute)).abs() < f32::EPSILON,
            "{attribute:?} holds the value its own arm wrote, got {held}",
        );
    }
}

#[test]
fn the_gang_name_answers_with_what_the_draft_holds_afterwards() {
    let mut draft = one_member();

    let written = write_field(
        &mut draft,
        GangFieldNet::Name(EditorDraftNameNet::new("Ash Ferals")),
    );

    assert_eq!(draft.name(), "Ash Ferals");
    assert_eq!(
        written,
        Ok(GangFieldNet::Name(EditorDraftNameNet::new(draft.name()))),
        "the reply carries the name read back off the draft, so a write that never landed shows",
    );
}

#[test]
fn a_member_weapon_answers_with_the_key_the_member_holds_afterwards() {
    let mut draft = one_member();

    let written = write_field(
        &mut draft,
        GangFieldNet::MemberWeapon {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );

    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert_eq!(
        member.weapon.as_ref().map(|key| key.as_str()),
        Some(SEEDED_KEY),
    );
    assert_eq!(
        written,
        Ok(GangFieldNet::MemberWeapon {
            index: FIRST,
            key:   member
                .weapon
                .as_ref()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        }),
        "the reply carries the key read back off the member",
    );
}

#[test]
fn a_member_armor_key_lands_on_the_member_the_index_names() {
    let mut draft = one_member();

    let written = write_field(
        &mut draft,
        GangFieldNet::MemberArmor {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );

    assert!(
        written.is_ok(),
        "a seeded armor key is written: {written:?}"
    );
    assert_eq!(
        draft
            .members()
            .first()
            .and_then(|member| member.armor.as_ref())
            .map(|key| key.as_str()),
        Some(SEEDED_KEY),
        "the member holds the armor key the write named, read off the draft",
    );
}

#[test]
fn a_member_armor_set_with_no_key_clears_the_key_and_answers_with_none() {
    let mut draft = one_member();
    let set = write_field(
        &mut draft,
        GangFieldNet::MemberArmor {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );
    assert!(set.is_ok(), "the seeded armor key is written: {set:?}");

    let cleared = write_field(
        &mut draft,
        GangFieldNet::MemberArmor {
            index: FIRST,
            key:   None,
        },
    );

    assert_eq!(
        cleared,
        Ok(GangFieldNet::MemberArmor {
            index: FIRST,
            key:   None,
        }),
        "the reply carries no key, read back off the member the write cleared",
    );
    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert_eq!(member.armor, None, "the cleared member holds no armor");
}

#[test]
fn a_member_melee_key_lands_on_the_member_the_index_names() {
    let mut draft = one_member();

    let written = write_field(
        &mut draft,
        GangFieldNet::MemberMeleeWeapon {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );

    assert!(
        written.is_ok(),
        "a seeded melee key is written: {written:?}"
    );
    assert_eq!(
        draft
            .members()
            .first()
            .and_then(|member| member.melee_weapon.as_ref())
            .map(|name| name.as_str()),
        Some(SEEDED_KEY),
        "the member holds the melee key the write named, read off the draft",
    );
}

#[test]
fn a_member_name_is_written_through_the_roster_the_panel_edits() {
    let mut draft = one_member();

    let written = write_field(
        &mut draft,
        GangFieldNet::MemberName {
            index: FIRST,
            name:  EditorDraftNameNet::new("Kez"),
        },
    );

    assert!(written.is_ok(), "a member name is written: {written:?}");
    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert_eq!(member.name.as_str(), "Kez");
}

#[test]
fn a_melee_key_is_cleared_to_the_fists_default_with_no_registry_in_the_world() {
    let mut draft = one_member();
    let set = write_field(
        &mut draft,
        GangFieldNet::MemberMeleeWeapon {
            index: FIRST,
            key:   Some(asked_key(SEEDED_KEY)),
        },
    );
    assert!(set.is_ok(), "the seeded melee key is written: {set:?}");

    let cleared = answered(write(
        &mut draft,
        Some(&weapons()),
        None,
        Some(&armor()),
        GangFieldNet::MemberMeleeWeapon {
            index: FIRST,
            key:   None,
        },
    ));

    assert_eq!(
        cleared,
        Ok(GangFieldNet::MemberMeleeWeapon {
            index: FIRST,
            key:   None,
        }),
        "clearing the melee pick consults no registry, so an absent one is no refusal",
    );
    let Some(member) = draft.members().first() else {
        unreachable!("the draft was built holding one member");
    };
    assert_eq!(member.melee_weapon, None);
}

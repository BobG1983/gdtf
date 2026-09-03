//! The Gang form's own field arms, driven over the listener with the Gang tab open.

use bevy::app::App;
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};
use gdtf_content_editor::EditorMode;

use crate::{
    rows::{FieldRow, GangFieldRow},
    setup::{form_tab_app_and_client, gang_draft, list_op, set_field},
    socket::Client,
    support::{TestError, TestResult},
    values::GangAttributeRow,
};

// The keys the member's weapon combo offers, in the order it offers them.
fn a_weapon_key(app: &App) -> Result<WeaponName, TestError> {
    let Some(registry) = app.world().get_resource::<WeaponRegistry>() else {
        return Err("the weapon registry is what the member's weapon combo reads".into());
    };
    let mut keys: Vec<WeaponName> = registry.keys().cloned().collect();
    keys.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    match keys.first() {
        Some(first) => Ok(first.clone()),
        None => Err("the weapon registry holds no key, so the combo offers no row".into()),
    }
}

// A Gang tab holding one member, which the roster's own Add button appends.
fn gang_tab_with_one_member() -> Result<(App, Client), TestError> {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;
    Ok((app, client))
}

#[test]
fn the_gang_name_reply_and_the_world_agree_on_the_frame_that_answered() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;

    let row = set_field(&mut app, &mut client, "(field: Gang(Name(\"Ash Ferals\")))")?;

    let held = gang_draft(&app)?;
    assert_eq!(
        row.field,
        FieldRow::Gang(GangFieldRow::Name(held.name().to_owned())),
        "the reply carries the name the draft holds on the frame that answered",
    );
    assert_eq!(held.name(), "Ash Ferals");
    Ok(())
}

#[test]
fn a_member_attribute_lands_on_the_member_the_index_names() -> TestResult {
    let (mut app, mut client) = gang_tab_with_one_member()?;

    let row = set_field(
        &mut app,
        &mut client,
        "(field: Gang(MemberAttribute(index: 0, attribute: Aim, value: 42.5)))",
    )?;

    let held = gang_draft(&app)?;
    let Some(member) = held.members().first() else {
        return Err("the Add left one member on the roster".into());
    };
    assert_eq!(
        row.field,
        FieldRow::Gang(GangFieldRow::MemberAttribute {
            index:     0,
            attribute: GangAttributeRow::Aim,
            value:     *member.aim,
        })
    );
    assert!(
        (*member.aim - 42.5).abs() < f32::EPSILON,
        "the member holds the aim the write asked for, got {}",
        *member.aim,
    );
    Ok(())
}

#[test]
fn a_member_weapon_takes_a_key_the_live_registry_holds() -> TestResult {
    let (mut app, mut client) = gang_tab_with_one_member()?;
    let key = a_weapon_key(&app)?;

    let row = set_field(
        &mut app,
        &mut client,
        &format!(
            "(field: Gang(MemberWeapon(index: 0, key: Some(\"{}\"))))",
            key.as_str()
        ),
    )?;

    let held = gang_draft(&app)?;
    let Some(member) = held.members().first() else {
        return Err("the Add left one member on the roster".into());
    };
    assert_eq!(
        row.field,
        FieldRow::Gang(GangFieldRow::MemberWeapon {
            index: 0,
            key:   member.weapon.as_ref().map(|k| k.as_str().to_owned()),
        })
    );
    assert_eq!(member.weapon, Some(key));
    Ok(())
}

#[test]
fn a_member_name_written_over_the_wire_reads_back_off_the_roster() -> TestResult {
    let (mut app, mut client) = gang_tab_with_one_member()?;

    let row = set_field(
        &mut app,
        &mut client,
        "(field: Gang(MemberName(index: 0, name: \"Kez\")))",
    )?;

    let held = gang_draft(&app)?;
    let Some(member) = held.members().first() else {
        return Err("the Add left one member on the roster".into());
    };
    assert_eq!(
        row.field,
        FieldRow::Gang(GangFieldRow::MemberName {
            index: 0,
            name:  member.name.as_str().to_owned(),
        })
    );
    assert_eq!(member.name.as_str(), "Kez");
    Ok(())
}

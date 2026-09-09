use gdtf_battle_sim::armor::ArmorType;

use crate::{
    mcp_editor_forms::{
        rows::{ListMemberRow, ListRow},
        setup::{field_draft, list_op, settled_field_app_and_client, try_list_op},
        values::ArmorTypeRow,
    },
    mcp_shared::{bad_arguments::bad_arguments_detail, support::TestResult},
};

const TOGGLE_FLAK: &str = "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))";

#[test]
fn toggling_an_immune_armor_type_adds_it_then_removes_it() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    assert!(
        field_draft(&app)?.immune_armor_types().is_empty(),
        "a blank Field draft's immune list is empty, which is legal and is where this starts",
    );

    let added = list_op(&mut app, &mut client, TOGGLE_FLAK)?;
    assert_eq!(added.list, ListRow::FieldImmuneArmorTypes);
    assert_eq!(
        added.members,
        vec![ListMemberRow::ImmuneArmorType(ArmorTypeRow::Flak)],
        "the reply reads the list back off the draft",
    );
    assert!(field_draft(&app)?.is_immune(ArmorType::Flak));

    let removed = list_op(&mut app, &mut client, TOGGLE_FLAK)?;
    assert!(
        removed.members.is_empty(),
        "the second toggle removes the type, so an add-only toggle leaves it listed and fails \
         here: {:?}",
        removed.members,
    );
    assert!(field_draft(&app)?.immune_armor_types().is_empty());
    Ok(())
}

#[test]
fn the_immune_list_reads_back_in_armor_type_order_whatever_order_it_was_toggled_in() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Hazard)))",
    )?;
    let both = list_op(&mut app, &mut client, TOGGLE_FLAK)?;
    assert_eq!(
        both.members,
        vec![
            ListMemberRow::ImmuneArmorType(ArmorTypeRow::Flak),
            ListMemberRow::ImmuneArmorType(ArmorTypeRow::Hazard),
        ],
        "the list reads back in ArmorType::ALL order, so a reply built from the set's hash \
         order fails here",
    );
    Ok(())
}

#[test]
fn every_op_but_toggle_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    list_op(&mut app, &mut client, TOGGLE_FLAK)?;
    let before = field_draft(&app)?;

    for op in [
        "Add",
        "Remove(0)",
        "MoveUp(0)",
        "MoveDown(0)",
        "SetAt(0, ImmuneArmorType(Void))",
    ] {
        let arguments = format!("(list: FieldImmuneArmorTypes, op: {op})");
        let reply = try_list_op(&mut app, &mut client, &arguments)?;
        let detail = bad_arguments_detail(&reply)?;
        assert!(
            detail.contains("tick box"),
            "`{op}` names an operation the tick-box row does not draw, and the detail says so: \
             `{detail}`",
        );
        assert_eq!(
            field_draft(&app)?,
            before,
            "the refused `{op}` left the Field draft exactly as it was",
        );
    }
    Ok(())
}

#[test]
fn a_member_from_another_forms_list_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    let before = field_draft(&app)?;

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(TerrainTag(Openable)))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        field_draft(&app)?,
        before,
        "a member that is not an armor type names no tick box, so nothing was written",
    );
    Ok(())
}

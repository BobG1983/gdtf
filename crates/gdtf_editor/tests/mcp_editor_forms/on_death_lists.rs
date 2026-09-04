//! The two on-death lists over `editor.list_op`: append, remove and reorder.

use gdtf_battle_sim::effects::on_death::OnDeathEffect;
use gdtf_editor::EditorMode;

use crate::{
    rows::{ListMemberRow, ListRow, OnDeathEffectRow},
    setup::{form_tab_app_and_client, list_op, set_field, terrain_draft},
    support::TestResult,
    values::{DamageTypeRow, HitTypeRow},
};

// The blast damage each row was written with, in the order the reply reads them back.
fn damages(members: &[ListMemberRow]) -> Vec<u16> {
    members
        .iter()
        .filter_map(|member| match member {
            ListMemberRow::OnDeathEffect(OnDeathEffectRow::Explode { damage, .. }) => Some(*damage),
            _ => None,
        })
        .collect()
}

// The blast damage each authored effect holds, in the draft's own order.
fn drafted_damages(effects: &[OnDeathEffect]) -> Vec<u16> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            OnDeathEffect::Explode { damage, .. } => Some(**damage),
            OnDeathEffect::LeaveField { .. } => None,
        })
        .collect()
}

#[test]
fn the_on_death_list_appends_removes_and_reorders() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;

    list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: Add)",
    )?;
    let added = list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: Add)",
    )?;
    assert_eq!(added.list, ListRow::TerrainOnDeathEffects);
    assert_eq!(added.members.len(), 2, "two Adds append two rows");

    set_field(
        &mut app,
        &mut client,
        "(field: Terrain(OnDeathDamage(index: 0, damage: 3)))",
    )?;
    set_field(
        &mut app,
        &mut client,
        "(field: Terrain(OnDeathDamage(index: 1, damage: 9)))",
    )?;
    assert_eq!(
        drafted_damages(terrain_draft(&app)?.on_death()),
        vec![3, 9],
        "each row is told apart by the damage it was written with, never by a position",
    );

    let moved = list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: MoveUp(1))",
    )?;
    assert_eq!(
        damages(&moved.members),
        vec![9, 3],
        "MoveUp(1) puts the row written second in front, and the reply reads the new order",
    );
    assert_eq!(
        drafted_damages(terrain_draft(&app)?.on_death()),
        vec![9, 3],
        "the draft agrees with the reply, so a reply-only reorder fails here",
    );

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: Remove(0))",
    )?;
    assert_eq!(
        damages(&removed.members),
        vec![3],
        "Remove(0) deletes the row MoveUp had just moved to the front, leaving the one written \
         FIRST",
    );
    assert_eq!(
        drafted_damages(terrain_draft(&app)?.on_death()),
        vec![3],
        "the draft holds that one effect",
    );
    Ok(())
}

#[test]
fn an_added_on_death_row_reads_back_as_the_forms_own_blank_explode() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;

    let added = list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;

    assert_eq!(added.list, ListRow::WeaponOnDeathEffects);
    assert_eq!(
        added.members,
        vec![ListMemberRow::OnDeathEffect(OnDeathEffectRow::Explode {
            hit_type:    HitTypeRow::Single,
            damage:      0,
            damage_type: DamageTypeRow::Kinetic,
        })],
        "the member carries the whole payload of the template the Add seeds",
    );
    Ok(())
}

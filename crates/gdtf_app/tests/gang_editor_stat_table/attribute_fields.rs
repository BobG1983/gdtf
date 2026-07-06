//! The expanded panel's eight clamped attribute fields + derived-stat recompute
//! through the pipeline (C2/C3).

use bevy::{
    app::App,
    ecs::entity::Entity,
    prelude::{Text, With},
};
use gdtf_app::test_support::{
    AttributeField, BaseAttribute, DerivedStat, DerivedStatText, EditableGang, MemberRowIndex,
};
use gdtf_battle_sim::{
    Aim, Cool, DerivedStats, GangerAttributes, Grit, Reflexes, Speed, Strength, Toughness,
    derive_stats, ganger::Luck,
};
use gdtf_ui::{CommittedNumericValue, NumericFieldCommitted, NumericRange};

use super::harness::*;

/// Every `AttributeField` entity carrying `MemberRowIndex == index`, paired with its
/// [`BaseAttribute`].
fn attribute_fields_for_row(app: &mut App, index: usize) -> Vec<(Entity, BaseAttribute)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &MemberRowIndex, &BaseAttribute), With<AttributeField>>();
    q.iter(app.world())
        .filter(|(_, row_index, _)| ***row_index == index)
        .map(|(entity, _, attribute)| (entity, *attribute))
        .collect()
}

/// The [`Text`] string of the readonly [`DerivedStatText`] node for `(index, stat)`.
fn derived_text(app: &mut App, index: usize, stat: DerivedStat) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&MemberRowIndex, &DerivedStat, &Text), With<DerivedStatText>>();
    q.iter(app.world())
        .find(|(row_index, kind, _)| ***row_index == index && **kind == stat)
        .map(|(_, _, text)| text.0.clone())
}

/// Format one derived stat the SAME way the production `derived_display` formatter does (the f32
/// skill stats to one decimal, the integer pools as bare integers) — kept in lockstep so the test
/// asserts against the production rendering of the SHARED `derive_stats` output (C3).
fn expected_derived(stats: &DerivedStats, stat: DerivedStat) -> String {
    match stat {
        DerivedStat::Shooting => format!("{:.1}", *stats.shooting),
        DerivedStat::Fight => format!("{:.1}", *stats.fight),
        DerivedStat::Reactions => format!("{:.1}", *stats.reactions),
        DerivedStat::Morale => format!("{:.1}", *stats.morale),
        DerivedStat::Tu => format!("{}", *stats.tu),
        DerivedStat::Hp => format!("{}", *stats.hp),
        DerivedStat::Wounds => format!("{}", *stats.wounds),
        DerivedStat::Bottle => format!("{}", *stats.bottle),
    }
}

/// A [`GangerAttributes`] record with every attribute at `value` — the editor seeds a fresh member
/// at `0.0`, so this builds the matching expected-derivation input.
const fn attributes_all(value: f32) -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(value),
        aim:       Aim::new(value),
        strength:  Strength::new(value),
        toughness: Toughness::new(value),
        reflexes:  Reflexes::new(value),
        cool:      Cool::new(value),
        grit:      Grit::new(value),
        luck:      Luck::new(value),
    }
}

/// C2: the expanded panel holds the EIGHT editable base-attribute numeric fields, each clamped via
/// a `NumericRange<f32>`.
///
/// Pin: a missing field (fewer than eight, or a missing attribute) fails the set assert; a field
/// without a clamp range fails the clamp assert.
#[test]
fn expanded_panel_has_eight_clamped_attribute_fields() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let fields = attribute_fields_for_row(&mut app, 0);
    assert_eq!(
        fields.len(),
        8,
        "the expanded panel must spawn one numeric field per editable base attribute (C2)",
    );
    // All eight distinct attributes are present.
    for attribute in BaseAttribute::ALL {
        assert!(
            fields.iter().any(|(_, kind)| *kind == attribute),
            "the panel must have a field for {attribute:?} (C2)",
        );
    }

    // Each field carries a NumericRange<f32> that actually clamps out-of-range input (C2).
    for (field, attribute) in fields {
        let range = app.world().get::<NumericRange<f32>>(field).copied();
        assert!(
            range.is_some(),
            "an attribute field for {attribute:?} must carry its NumericRange<f32> clamp (C2)",
        );
        // Default range is degenerate; the assert above already failed if the real range is absent.
        let range = range.unwrap_or_else(|| NumericRange::new(0.0_f32, 0.0_f32));
        // The clamp returns EXACTLY the bound (same float), so compare bits to dodge the f32
        // strict-comparison lint — this is an exact-equality assertion, not a tolerance one.
        let above = range.max() + 1_000.0;
        assert_eq!(
            range.clamp(above).to_bits(),
            range.max().to_bits(),
            "the {attribute:?} field's range must clamp an above-max value down to max (C2)",
        );
        let below = range.min() - 1_000.0;
        assert_eq!(
            range.clamp(below).to_bits(),
            range.min().to_bits(),
            "the {attribute:?} field's range must clamp a below-min value up to min (C2)",
        );
    }
}

/// C3: a real `NumericFieldCommitted<f32>` on a member's attribute field updates that member's
/// attribute AND every displayed derived stat equals the GTW-384 `derive_stats` output for the
/// member's NEW attributes — proving the displayed value == pipeline(attrs).
///
/// Pin-discriminating: dropping the recompute leaves the OLD derived text (which differs, asserted
/// via the precondition), and using STALE attributes (deriving before the set) would also fail the
/// equality. The test computes its expected through the SAME `derive_stats` pipeline + tuning the
/// production code uses — it does NOT reimplement the derivation.
#[test]
fn editing_attribute_recomputes_derived_to_pipeline_output() {
    let mut app = editor_app();
    press_add_member(&mut app);

    // A fresh member's attributes are all 0.0; its derived stats are the pipeline output for the
    // all-zero attribute record. Edit Grit to a value that visibly moves HP / Wounds / Morale.
    let grit_field = {
        let fields = attribute_fields_for_row(&mut app, 0);
        fields
            .into_iter()
            .find(|(_, attribute)| *attribute == BaseAttribute::Grit)
            .map_or(Entity::PLACEHOLDER, |(entity, _)| entity)
    };

    // Precondition: HP currently reads the all-zero derivation (so the post-edit value differs).
    let zero_stats = derive_stats(&attributes_all(0.0), &test_tuning());
    assert_eq!(
        derived_text(&mut app, 0, DerivedStat::Hp).as_deref(),
        Some(expected_derived(&zero_stats, DerivedStat::Hp).as_str()),
        "precondition: a fresh member's HP display reads the all-zero pipeline output",
    );

    // Commit a new Grit via the REAL numeric-field commit message the widget raises.
    let new_grit = 8.0_f32;
    app.world_mut().write_message(NumericFieldCommitted::new(
        grit_field,
        CommittedNumericValue::new(new_grit),
    ));
    app.update();

    // The model attribute updated (C3) — compare bits (exact-equality, dodges the f32 lint).
    let model_grit = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| m.attribute(BaseAttribute::Grit)));
    assert_eq!(
        model_grit.map(f32::to_bits),
        Some(new_grit.to_bits()),
        "the commit must set THAT member's Grit attribute in the model (C3)",
    );

    // The expected derived stats come from the SAME pipeline + tuning the production code uses.
    let mut new_attrs = attributes_all(0.0);
    new_attrs.grit = Grit::new(new_grit);
    let expected = derive_stats(&new_attrs, &test_tuning());

    // EVERY displayed derived stat equals the pipeline output for the new attributes (C3).
    for stat in DerivedStat::ALL {
        assert_eq!(
            derived_text(&mut app, 0, stat).as_deref(),
            Some(expected_derived(&expected, stat).as_str()),
            "the displayed {stat:?} must equal the GTW-384 pipeline output for the live attributes \
             (C3: displayed == pipeline(attrs))",
        );
    }

    // The discriminator: HP actually MOVED off its all-zero value (so the assert is not vacuous).
    assert_ne!(
        expected_derived(&expected, DerivedStat::Hp),
        expected_derived(&zero_stats, DerivedStat::Hp),
        "the edit must change the derived HP (else the recompute assert would be vacuous)",
    );
}

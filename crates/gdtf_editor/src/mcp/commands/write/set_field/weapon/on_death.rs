//! The Weapon form's on-death rows, each reached through its index in the list.

use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DamageType, HitType, WeaponSpec},
};

use crate::{
    mcp::{
        commands::write::{form_fault::FormWriteFault, list_op::shared::past_the_end},
        wire::{
            DamageTypeNet, EditorListIndexNet, EditorListNet, ExplodeDamageNet, FieldKeyNet,
            HitTypeNet, OnDeathVariantNet, WeaponFieldNet,
        },
    },
    weapon_form::{explode_template, leave_field_template},
};

const NOT_EXPLODING: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's on death effect at that index is LeaveField, so the form draws no hit \
     type, no blast damage and no blast damage channel",
);

const NOT_LEAVING_A_FIELD: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's on death effect at that index is Explode, so the form draws no field key",
);

// The effect at an index, or the fault a past-the-end index answers.
fn effect_at(
    spec: &WeaponSpec,
    index: EditorListIndexNet,
) -> Result<OnDeathEffect, FormWriteFault> {
    match spec.on_death.get(*index) {
        Some(effect) => Ok(effect.clone()),
        None => Err(past_the_end(
            EditorListNet::WeaponOnDeathEffects,
            *index,
            spec.on_death.len(),
        )),
    }
}

// Rewrite the explode payload the form draws, or refuse for the variant the row is not on.
fn with_explode<R>(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    edit: impl FnOnce(&mut HitType, &mut ExplodeDamage, &mut DamageType) -> R,
) -> Result<R, FormWriteFault> {
    let held = spec.on_death.len();
    let Some(effect) = spec.on_death.get_mut(*index) else {
        return Err(past_the_end(
            EditorListNet::WeaponOnDeathEffects,
            *index,
            held,
        ));
    };
    let OnDeathEffect::Explode {
        hit_type,
        damage,
        damage_type,
    } = effect
    else {
        return Err(FormWriteFault::Gated(NOT_EXPLODING));
    };
    Ok(edit(hit_type, damage, damage_type))
}

// Rewrite the leave-field payload the form draws, or refuse for the variant the row is not on.
fn with_field_key<R>(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    edit: impl FnOnce(&mut FieldKey) -> R,
) -> Result<R, FormWriteFault> {
    let held = spec.on_death.len();
    let Some(effect) = spec.on_death.get_mut(*index) else {
        return Err(past_the_end(
            EditorListNet::WeaponOnDeathEffects,
            *index,
            held,
        ));
    };
    let OnDeathEffect::LeaveField { field } = effect else {
        return Err(FormWriteFault::Gated(NOT_LEAVING_A_FIELD));
    };
    Ok(edit(field))
}

/// Swap one row for the other variant's blank template, the way the form's combo does.
pub(super) fn variant(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    wanted: OnDeathVariantNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    let held = effect_at(spec, index)?;
    if OnDeathVariantNet::from_effect(&held) != wanted {
        let swapped = match wanted {
            OnDeathVariantNet::Explode => explode_template(),
            OnDeathVariantNet::LeaveField => leave_field_template(),
        };
        if let Some(slot) = spec.on_death.get_mut(*index) {
            *slot = swapped;
        }
    }
    Ok(WeaponFieldNet::OnDeathVariant {
        index,
        variant: OnDeathVariantNet::from_effect(&effect_at(spec, index)?),
    })
}

/// Write one row's hit geometry, answering the value the effect stores.
pub(super) fn hit_type(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    wanted: HitTypeNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, index, |hit_type, _, _| {
        *hit_type = wanted.to_hit_type();
        WeaponFieldNet::OnDeathHitType {
            index,
            hit_type: HitTypeNet::from_hit_type(*hit_type),
        }
    })
}

/// Write one row's blast damage, answering the value the effect stores.
pub(super) fn damage(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    wanted: ExplodeDamageNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, index, |_, damage, _| {
        *damage = wanted.to_damage();
        WeaponFieldNet::OnDeathDamage {
            index,
            damage: ExplodeDamageNet::new(**damage),
        }
    })
}

/// Write one row's damage channel, answering the value the effect stores.
pub(super) fn damage_type(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    wanted: DamageTypeNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, index, |_, _, damage_type| {
        *damage_type = wanted.to_damage_type();
        WeaponFieldNet::OnDeathDamageType {
            index,
            damage_type: DamageTypeNet::from_damage_type(*damage_type),
        }
    })
}

/// Write one row's field key, answering the value the effect stores.
pub(super) fn field(
    spec: &mut WeaponSpec,
    index: EditorListIndexNet,
    wanted: FieldKeyNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_field_key(spec, index, |field| {
        *field = wanted.to_key();
        WeaponFieldNet::OnDeathField {
            index,
            field: FieldKeyNet::from_key(field),
        }
    })
}

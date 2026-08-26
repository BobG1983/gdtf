//! The Weapon form's on-death tick box, its variant combo, and each variant's own payload.

use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DamageType, HitType, WeaponSpec},
};
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            DamageTypeNet, EditorFieldNet, ExplodeDamageNet, FieldKeyNet, HitTypeNet,
            OnDeathEnabledNet, OnDeathVariantNet,
        },
    },
    weapon_form::{explode_template, leave_field_template},
};

const ON_DEATH_OFF: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's on death effect tick box is off, and the variant and its payload are \
     drawn only while it is on",
);

const NOT_EXPLODING: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's on death effect is LeaveField, so the form draws no hit type, no blast \
     damage and no blast damage channel",
);

const NOT_LEAVING_A_FIELD: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's on death effect is Explode, so the form draws no field key",
);

// Reach the authored effect, or refuse for the tick box that is off.
fn with_effect<R>(
    spec: &mut WeaponSpec,
    edit: impl FnOnce(&mut OnDeathEffect) -> R,
) -> Result<R, FormWriteFault> {
    match spec.on_death.as_mut() {
        Some(effect) => Ok(edit(effect)),
        None => Err(FormWriteFault::Gated(ON_DEATH_OFF)),
    }
}

// Reach the explode payload the form draws, or refuse for the variant the draft is not on.
fn with_explode<R>(
    spec: &mut WeaponSpec,
    edit: impl FnOnce(&mut HitType, &mut ExplodeDamage, &mut DamageType) -> R,
) -> Result<R, FormWriteFault> {
    with_effect(spec, |effect| match effect {
        OnDeathEffect::Explode {
            hit_type,
            damage,
            damage_type,
        } => Ok(edit(hit_type, damage, damage_type)),
        OnDeathEffect::LeaveField { .. } => Err(FormWriteFault::Gated(NOT_EXPLODING)),
    })?
}

// Reach the leave-field payload the form draws, or refuse for the variant the draft is not on.
fn with_field_key<R>(
    spec: &mut WeaponSpec,
    edit: impl FnOnce(&mut FieldKey) -> R,
) -> Result<R, FormWriteFault> {
    with_effect(spec, |effect| match effect {
        OnDeathEffect::LeaveField { field } => Ok(edit(field)),
        OnDeathEffect::Explode { .. } => Err(FormWriteFault::Gated(NOT_LEAVING_A_FIELD)),
    })?
}

// Swap the effect for the other variant's blank template, the way the form's combo does.
fn pick_variant(
    spec: &mut WeaponSpec,
    wanted: OnDeathVariantNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    with_effect(spec, |effect| {
        if OnDeathVariantNet::from_effect(effect) != wanted {
            *effect = match wanted {
                OnDeathVariantNet::Explode => explode_template(),
                OnDeathVariantNet::LeaveField => leave_field_template(),
            };
        }
        EditorFieldNet::WeaponOnDeathVariant(OnDeathVariantNet::from_effect(effect))
    })
}

/// Write the on-death tick box, its variant, or one payload row, as the effect stores it.
pub(super) fn write(
    spec: &mut WeaponSpec,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponOnDeath(enabled) => {
            spec.on_death = enabled.is_enabled().then(explode_template);
            Ok(EditorFieldNet::WeaponOnDeath(OnDeathEnabledNet::from_spec(
                spec,
            )))
        }
        EditorFieldNet::WeaponOnDeathVariant(wanted) => pick_variant(spec, wanted),
        EditorFieldNet::WeaponOnDeathHitType(wanted) => with_explode(spec, |hit_type, _, _| {
            *hit_type = wanted.to_hit_type();
            EditorFieldNet::WeaponOnDeathHitType(HitTypeNet::from_hit_type(*hit_type))
        }),
        EditorFieldNet::WeaponOnDeathDamage(wanted) => with_explode(spec, |_, damage, _| {
            *damage = wanted.to_damage();
            EditorFieldNet::WeaponOnDeathDamage(ExplodeDamageNet::new(**damage))
        }),
        EditorFieldNet::WeaponOnDeathDamageType(wanted) => {
            with_explode(spec, |_, _, damage_type| {
                *damage_type = wanted.to_damage_type();
                EditorFieldNet::WeaponOnDeathDamageType(DamageTypeNet::from_damage_type(
                    *damage_type,
                ))
            })
        }
        EditorFieldNet::WeaponOnDeathField(wanted) => with_field_key(spec, |field| {
            *field = wanted.to_key();
            EditorFieldNet::WeaponOnDeathField(FieldKeyNet::from_key(field))
        }),
        _ => Err(FormWriteFault::ForeignArm),
    }
}

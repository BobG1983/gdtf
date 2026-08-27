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
            DamageTypeNet, ExplodeDamageNet, FieldKeyNet, HitTypeNet, OnDeathEnabledNet,
            OnDeathVariantNet, WeaponFieldNet,
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

/// Turn the on-death effect on or off, answering the state the spec is left in.
pub(super) fn enabled(spec: &mut WeaponSpec, enabled: OnDeathEnabledNet) -> WeaponFieldNet {
    spec.on_death = enabled.is_enabled().then(explode_template);
    WeaponFieldNet::OnDeath(OnDeathEnabledNet::from_spec(spec))
}

/// Swap the effect for the other variant's blank template, the way the form's combo does.
pub(super) fn variant(
    spec: &mut WeaponSpec,
    wanted: OnDeathVariantNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_effect(spec, |effect| {
        if OnDeathVariantNet::from_effect(effect) != wanted {
            *effect = match wanted {
                OnDeathVariantNet::Explode => explode_template(),
                OnDeathVariantNet::LeaveField => leave_field_template(),
            };
        }
        WeaponFieldNet::OnDeathVariant(OnDeathVariantNet::from_effect(effect))
    })
}

/// Write the explode payload's hit geometry, answering the value the effect stores.
pub(super) fn hit_type(
    spec: &mut WeaponSpec,
    wanted: HitTypeNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, |hit_type, _, _| {
        *hit_type = wanted.to_hit_type();
        WeaponFieldNet::OnDeathHitType(HitTypeNet::from_hit_type(*hit_type))
    })
}

/// Write the explode payload's blast damage, answering the value the effect stores.
pub(super) fn damage(
    spec: &mut WeaponSpec,
    wanted: ExplodeDamageNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, |_, damage, _| {
        *damage = wanted.to_damage();
        WeaponFieldNet::OnDeathDamage(ExplodeDamageNet::new(**damage))
    })
}

/// Write the explode payload's damage channel, answering the value the effect stores.
pub(super) fn damage_type(
    spec: &mut WeaponSpec,
    wanted: DamageTypeNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_explode(spec, |_, _, damage_type| {
        *damage_type = wanted.to_damage_type();
        WeaponFieldNet::OnDeathDamageType(DamageTypeNet::from_damage_type(*damage_type))
    })
}

/// Write the leave-field payload's field key, answering the value the effect stores.
pub(super) fn field(
    spec: &mut WeaponSpec,
    wanted: FieldKeyNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_field_key(spec, |field| {
        *field = wanted.to_key();
        WeaponFieldNet::OnDeathField(FieldKeyNet::from_key(field))
    })
}

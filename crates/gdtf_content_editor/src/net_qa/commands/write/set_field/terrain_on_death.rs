//! The Terrain form's on-death rows, each reached through its index in the list.

use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DamageType, HitType},
};
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    net_qa::{
        commands::write::{form_fault::FormWriteFault, list_op::shared::past_the_end},
        wire::{
            DamageTypeNet, EditorListIndexNet, EditorListNet, ExplodeDamageNet, FieldKeyNet,
            HitTypeNet, OnDeathVariantNet, TerrainFieldNet,
        },
    },
    terrain_form::TerrainDraft,
    weapon_form::{explode_template, leave_field_template},
};

const NOT_EXPLODING: RefusalNote = RefusalNote::from_static(
    "the Terrain draft's on death effect at that index is LeaveField, so the form draws no hit \
     type, no blast damage and no blast damage channel",
);

const NOT_LEAVING_A_FIELD: RefusalNote = RefusalNote::from_static(
    "the Terrain draft's on death effect at that index is Explode, so the form draws no field key",
);

// The effect at an index, or the fault a past-the-end index answers.
fn effect_at(
    draft: &TerrainDraft,
    index: EditorListIndexNet,
) -> Result<OnDeathEffect, FormWriteFault> {
    match draft.on_death().get(*index) {
        Some(effect) => Ok(effect.clone()),
        None => Err(past_the_end(
            EditorListNet::TerrainOnDeathEffects,
            *index,
            draft.on_death().len(),
        )),
    }
}

// Rewrite the explode payload the form draws, or refuse for the variant the row is not on.
fn with_explode<R>(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    edit: impl FnOnce(&mut HitType, &mut ExplodeDamage, &mut DamageType) -> R,
) -> Result<R, FormWriteFault> {
    let mut effect = effect_at(draft, index)?;
    let OnDeathEffect::Explode {
        hit_type,
        damage,
        damage_type,
    } = &mut effect
    else {
        return Err(FormWriteFault::Gated(NOT_EXPLODING));
    };
    let answered = edit(hit_type, damage, damage_type);
    draft.set_on_death_at(*index, effect);
    Ok(answered)
}

// Rewrite the leave-field payload the form draws, or refuse for the variant the row is not on.
fn with_field_key<R>(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    edit: impl FnOnce(&mut FieldKey) -> R,
) -> Result<R, FormWriteFault> {
    let mut effect = effect_at(draft, index)?;
    let OnDeathEffect::LeaveField { field } = &mut effect else {
        return Err(FormWriteFault::Gated(NOT_LEAVING_A_FIELD));
    };
    let answered = edit(field);
    draft.set_on_death_at(*index, effect);
    Ok(answered)
}

/// Swap one row for the other variant's blank template, the way the form's combo does.
pub(super) fn variant(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    wanted: OnDeathVariantNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    let effect = effect_at(draft, index)?;
    if OnDeathVariantNet::from_effect(&effect) != wanted {
        let swapped = match wanted {
            OnDeathVariantNet::Explode => explode_template(),
            OnDeathVariantNet::LeaveField => leave_field_template(),
        };
        draft.set_on_death_at(*index, swapped);
    }
    Ok(TerrainFieldNet::OnDeathVariant {
        index,
        variant: OnDeathVariantNet::from_effect(&effect_at(draft, index)?),
    })
}

/// Write one row's hit geometry, answering the value the effect stores.
pub(super) fn hit_type(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    wanted: HitTypeNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    with_explode(draft, index, |hit_type, _, _| {
        *hit_type = wanted.to_hit_type();
        TerrainFieldNet::OnDeathHitType {
            index,
            hit_type: HitTypeNet::from_hit_type(*hit_type),
        }
    })
}

/// Write one row's blast damage, answering the value the effect stores.
pub(super) fn damage(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    wanted: ExplodeDamageNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    with_explode(draft, index, |_, damage, _| {
        *damage = wanted.to_damage();
        TerrainFieldNet::OnDeathDamage {
            index,
            damage: ExplodeDamageNet::new(**damage),
        }
    })
}

/// Write one row's damage channel, answering the value the effect stores.
pub(super) fn damage_type(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    wanted: DamageTypeNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    with_explode(draft, index, |_, _, damage_type| {
        *damage_type = wanted.to_damage_type();
        TerrainFieldNet::OnDeathDamageType {
            index,
            damage_type: DamageTypeNet::from_damage_type(*damage_type),
        }
    })
}

/// Write one row's field key, answering the value the effect stores.
pub(super) fn field(
    draft: &mut TerrainDraft,
    index: EditorListIndexNet,
    wanted: FieldKeyNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    with_field_key(draft, index, |field| {
        *field = wanted.to_key();
        TerrainFieldNet::OnDeathField {
            index,
            field: FieldKeyNet::from_key(field),
        }
    })
}

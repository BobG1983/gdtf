//! The Weapon form's damage-over-time tick box and the three rows behind it.

use gdtf_battle_sim::weapon::{DotProfile, WeaponSpec};
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{DamageTypeNet, DotDamageNet, DotEnabledNet, DotTurnsNet, EditorFieldNet},
    },
    weapon_form::dot_turns_from_raw,
};

const DOT_OFF: RefusalNote = RefusalNote::from_static(
    "the Weapon draft's dot tick box is off, and the per-turn damage, the turns and the damage \
     channel are drawn only while it is on",
);

// Reach the authored profile the three rows edit, or refuse for the tick box that is off.
fn with_profile<R>(
    spec: &mut WeaponSpec,
    edit: impl FnOnce(&mut DotProfile) -> R,
) -> Result<R, FormWriteFault> {
    match spec.dot.as_mut() {
        Some(profile) => Ok(edit(profile)),
        None => Err(FormWriteFault::Gated(DOT_OFF)),
    }
}

/// Write the dot tick box or one of its rows, answering the value as the profile stores it.
pub(super) fn write(
    spec: &mut WeaponSpec,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponDot(enabled) => {
            spec.dot = enabled.is_enabled().then(DotProfile::default);
            Ok(EditorFieldNet::WeaponDot(DotEnabledNet::from_spec(spec)))
        }
        EditorFieldNet::WeaponDotDamage(damage) => with_profile(spec, |profile| {
            profile.damage = damage.to_damage();
            EditorFieldNet::WeaponDotDamage(DotDamageNet::new(*profile.damage))
        }),
        EditorFieldNet::WeaponDotTurns(turns) => with_profile(spec, |profile| {
            profile.turns = dot_turns_from_raw(*turns);
            EditorFieldNet::WeaponDotTurns(DotTurnsNet::new(profile.turns.get()))
        }),
        EditorFieldNet::WeaponDotDamageType(damage_type) => with_profile(spec, |profile| {
            profile.damage_type = damage_type.to_damage_type();
            EditorFieldNet::WeaponDotDamageType(DamageTypeNet::from_damage_type(
                profile.damage_type,
            ))
        }),
        _ => Err(FormWriteFault::ForeignArm),
    }
}

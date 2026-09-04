//! The Weapon form's damage-over-time tick box and the three rows behind it.

use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::weapon::{DotProfile, WeaponSpec};

use crate::{
    mcp::{
        commands::write::form_fault::FormWriteFault,
        wire::{DamageTypeNet, DotDamageNet, DotEnabledNet, DotTurnsNet, WeaponFieldNet},
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

/// Turn the dot profile on or off, answering the state the spec is left in.
pub(super) fn enabled(spec: &mut WeaponSpec, enabled: DotEnabledNet) -> WeaponFieldNet {
    spec.dot = enabled.is_enabled().then(DotProfile::default);
    WeaponFieldNet::Dot(DotEnabledNet::from_spec(spec))
}

/// Write the profile's per-turn damage, answering the value it stores.
pub(super) fn damage(
    spec: &mut WeaponSpec,
    damage: DotDamageNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_profile(spec, |profile| {
        profile.damage = damage.to_damage();
        WeaponFieldNet::DotDamage(DotDamageNet::new(*profile.damage))
    })
}

/// Write the profile's duration, answering the count it stores after the form's own zero fix.
pub(super) fn turns(
    spec: &mut WeaponSpec,
    turns: DotTurnsNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_profile(spec, |profile| {
        profile.turns = dot_turns_from_raw(*turns);
        WeaponFieldNet::DotTurns(DotTurnsNet::new(profile.turns.get()))
    })
}

/// Write the profile's damage channel, answering the value it stores.
pub(super) fn damage_type(
    spec: &mut WeaponSpec,
    damage_type: DamageTypeNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    with_profile(spec, |profile| {
        profile.damage_type = damage_type.to_damage_type();
        WeaponFieldNet::DotDamageType(DamageTypeNet::from_damage_type(profile.damage_type))
    })
}

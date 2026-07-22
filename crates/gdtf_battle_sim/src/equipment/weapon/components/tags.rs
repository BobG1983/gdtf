//! Connect-behavior boolean tags — the [`Shove`] knock-back tag and the
//! [`Silenced`] suppressor tag.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A weapon's **`shove` tag** — whether the weapon KNOCKS BACK the target on a
/// connecting attack (GTW-525). A `shove` weapon (a shock maul, a heavy bolter's
/// muzzle-thump, a boarding shield) AUTO-shoves its target one cell directly away
/// from the attacker on EVERY connecting hit — a MELEE strike connect OR a RANGED
/// shot connect — in ADDITION to the attack's damage. A miss does not shove; a
/// non-`shove` weapon never shoves.
///
/// The shove is PURE DISPLACEMENT: the tag adds no wound of its own (the attack's
/// own damage stands; any FALL the displacement triggers does the extra harm,
/// through the shared GTW-523 fall path). It mirrors the [`Stable`](super::ballistics::Stable) tag exactly — a
/// data-driven boolean MARKER set in the weapon `.ron`, present on only SOME
/// weapons; unlike [`Stable`](super::ballistics::Stable) it lives on BOTH the ranged AND the melee weapon
/// model (any weapon can knock back).
///
/// A weapon NUMBER (a boolean flag, lives on the weapon, not in tuning). A named
/// newtype (no-bare-types: a `bool` carrying domain meaning is wrapped). Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
/// entity, the [`Stable`](super::ballistics::Stable) precedent.
/// `Default` (`Shove(false)`) is BOTH the spawn-seed sentinel (the `bsn!` spawn path
/// seeds the slot via `Default` before an authored `Shove::new(..)` overwrites it,
/// GTW-322) AND the defensible authored default: a weapon authored WITHOUT a `shove:`
/// field is a NON-shove weapon (the field is `#[serde(default)]` on the spec, so the
/// vast majority of existing weapons that never author it keep shoving off). Unlike
/// [`Stable`](super::ballistics::Stable) — a required RON field — `shove` is opt-in.
/// [`Serialize`] so the editor's WEAPON mode saves the field in the same schema it
/// loads (GTW-670).
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Shove(bool);

impl Shove {
    /// Build a `shove` tag from its boolean value (`true` = knocks the target back
    /// one cell on a connecting hit).
    #[must_use]
    pub const fn new(shove: bool) -> Self {
        Self(shove)
    }
}

/// A weapon's **`silenced` tag** — whether the weapon carries a suppressor / silencer
/// fitted as a GTW-542 attachment. A `silenced` weapon fires QUIETLY: its shots do
/// NOT propagate the two "loud" battle signals a normal shot does —
///
/// - **suppression** — a silenced shot never PINS opposing gangers (the GTW-526
///   [`apply_suppression`](crate::suppression::apply_suppression) producer skips a silenced
///   shooter's [`FireRequested`](crate::acts::FireRequested)); and
/// - **reaction/reveal** — a silenced shot never TRIPS an opposing reactor's
///   interrupt (the GTW-468 [`reaction_trigger`](crate::reaction::reaction_trigger)
///   producer skips a [`FireDeclaration`](crate::acts::FireDeclaration) whose shooter
///   wields a silenced weapon, including a silenced INTERRUPT shot — a silenced shot
///   stays silent even when it is itself a reaction).
///
/// The shot still resolves its damage normally; only its NOISE footprint is removed.
///
/// A named newtype (no-bare-types: a `bool` carrying domain meaning is wrapped).
/// Private inner + derived [`Deref`]; `#[serde(transparent)]`. A `#[derive(Component)]`
/// (GTW-200) — a sibling component on the armed entity, present ONLY when a suppressor
/// is fitted; both producers gate on its presence via `shooter → Wields → weapon →
/// Option<&Silenced>` (absent = a normal, LOUD shot, identical to before this
/// tag).
/// `Default` (`Silenced(true)`) is the `bsn!` spawn-seed sentinel (GTW-322); the
/// component is inserted only when a suppressor is fitted, so `true` is the only
/// meaningful value, overwritten by the folder-fn's `Silenced::new(true)` regardless.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Silenced(bool);

impl Silenced {
    /// Build a `silenced` tag from its boolean value (`true` = a suppressor is fitted,
    /// so the shot propagates neither suppression nor reaction/reveal).
    #[must_use]
    pub const fn new(silenced: bool) -> Self {
        Self(silenced)
    }
}

impl Default for Silenced {
    /// The spawn-seed sentinel (GTW-322): `Silenced(true)`. The component is inserted
    /// only when a suppressor attachment is fitted, so `true` is the only meaningful
    /// value; the folder-fn overwrites this seed regardless.
    fn default() -> Self {
        Self(true)
    }
}

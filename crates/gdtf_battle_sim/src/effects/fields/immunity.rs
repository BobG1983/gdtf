//! The **whole-armor immunity** field consequence (GTW-545; GTW-553
//! one-file-per-consequence) — the [`ImmuneArmorTypes`] payload newtype and the isolated
//! [`ApplyImmunity`] behaviour that exempts a ganger from the field's drain when ANY of
//! its worn armor pieces carries an immune [`ArmorType`] (the sealed suit protects you).

use bevy::platform::collections::HashSet;
use serde::Deserialize;

use super::{ApplyFieldEffect, OccupantArmor};
use crate::armor::ArmorType;

/// The set of [`ArmorType`]s that make a ganger **wholly immune** to a field — the GTW-545
/// whole-source-immunity mechanism (`docs/combat/resolution.md` — the area-damage-field beat).
///
/// A ganger ANY of whose worn armor pieces carries an [`ArmorType`] in this set takes ZERO
/// damage from the field (the sealed suit protects you) — a whole-SOURCE skip, distinct from
/// the per-hit armor matchup (which soaks partial damage per wheel node). The seven-node
/// damage wheel has no whole-source-skip equivalent today; this is the new mechanism.
///
/// A named newtype [`HashSet`] (no-bare-types: the immune set is a domain value, not a bare
/// `HashSet<ArmorType>` in a `pub` field). Private inner with a [`contains`](ImmuneArmorTypes::contains)
/// accessor (the set answers a membership question, not a raw-set one). Derives
/// [`Deserialize`] so an authored `.ron` names it as a bare RON list of [`ArmorType`] variants
/// (`immune_armor_types: [Flak, Hazard]`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ImmuneArmorTypes(HashSet<ArmorType>);

impl ImmuneArmorTypes {
    /// Build an immune-armor-type set from an [`ArmorType`] iterator — the shape a test (and a
    /// field spawn call site) constructs.
    #[must_use]
    pub fn new(types: impl IntoIterator<Item = ArmorType>) -> Self {
        Self(types.into_iter().collect())
    }

    /// Whether this set contains `armor_type` — the membership check the
    /// [`ApplyImmunity`] exemption verb runs per worn armor piece to decide whole-source
    /// immunity (a match on ANY worn piece skips the field's drain entirely).
    #[must_use]
    pub fn contains(&self, armor_type: &ArmorType) -> bool {
        self.0.contains(armor_type)
    }

    /// Whether this set is empty (no armor type grants immunity) — the common case for a
    /// field nothing protects against.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// **Immunity** — the whole-armor exemption field consequence (GTW-545; isolated per
/// GTW-553).
///
/// Answers the pre-drain gate: if ANY of the occupant's worn armor pieces carries an
/// [`ArmorType`] in the field's [`ImmuneArmorTypes`] set, the occupant is EXEMPT — it
/// takes ZERO damage this round, with NO per-hit armor matchup, NO injury roll, and NO
/// RNG (the deterministic field tick). Each worn piece's type is resolved through the
/// borrowed [`OccupantArmor`] surface's worn-piece lookup (the melee resolve precedent).
///
/// BORROWS its set from the vocabulary variant (the delegation arm lends
/// `&ImmuneArmorTypes` for the duration of one gate — the set owns a [`HashSet`], and the
/// gate never needs to keep it; the [`ApplyLeaveField`](crate::effects::on_death::ApplyLeaveField)
/// borrowed-payload precedent).
pub struct ApplyImmunity<'s> {
    /// The armor types that grant whole-source immunity.
    armor_types: &'s ImmuneArmorTypes,
}

impl<'s> ApplyImmunity<'s> {
    /// Build the immunity gate borrowing its authored immune set.
    #[must_use]
    pub const fn new(armor_types: &'s ImmuneArmorTypes) -> Self {
        Self { armor_types }
    }
}

impl ApplyFieldEffect for ApplyImmunity<'_> {
    /// Exempt the occupant iff ANY worn piece's [`ArmorType`] is in the immune set —
    /// resolved piece-by-piece through the surface's worn-armor lookup. A piece the
    /// lookup cannot resolve (a stale relationship) counts as NOT immune (fail-closed:
    /// the field still drains).
    fn exempts_occupant(&self, armor: &OccupantArmor<'_, '_, '_>) -> bool {
        armor.wears.pieces().any(|piece| {
            armor
                .worn
                .get(piece)
                .is_ok_and(|armor_type| self.armor_types.contains(armor_type))
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::SystemState,
        prelude::{Query, With, World},
    };

    use super::{ApplyImmunity, ImmuneArmorTypes};
    use crate::{
        armor::{ArmorType, Wears, WornBy},
        effects::fields::{ApplyFieldEffect, OccupantArmor},
    };

    /// Whether an occupant wearing ONE piece of `worn_type` is exempted by a field whose
    /// immune set is `immune`. Bare-`World` + `SystemState` is the sanctioned pure-sim
    /// unit-test idiom (`bevy-traps.md` #7 carve-out (b)); the worn piece relates back
    /// via [`WornBy`], populating the ganger's [`Wears`] collection.
    fn exempts(worn_type: ArmorType, immune: &[ArmorType]) -> bool {
        let mut world = World::new();
        let ganger = world.spawn_empty().id();
        world.spawn((worn_type, WornBy::new(ganger)));
        let mut state: SystemState<Query<&'static ArmorType, With<WornBy>>> =
            SystemState::new(&mut world);
        let Ok(worn) = state.get(&world) else {
            unreachable!("a plain read-only Query SystemParam always validates");
        };
        let Some(wears) = world.get::<Wears>(ganger) else {
            unreachable!("spawning the WornBy piece populates the ganger's Wears");
        };
        let armor = OccupantArmor { wears, worn: &worn };
        let set = ImmuneArmorTypes::new(immune.iter().copied());
        ApplyImmunity::new(&set).exempts_occupant(&armor)
    }

    /// A worn piece whose type IS in the immune set exempts the occupant.
    #[test]
    fn a_matching_worn_armor_type_exempts_the_occupant() {
        assert!(
            exempts(ArmorType::Flak, &[ArmorType::Flak, ArmorType::Hazard]),
            "ANY worn piece in the immune set grants whole-source immunity"
        );
    }

    /// A worn piece whose type is NOT in the immune set does not exempt.
    #[test]
    fn a_non_matching_worn_armor_type_does_not_exempt() {
        assert!(
            !exempts(ArmorType::Plated, &[ArmorType::Flak, ArmorType::Hazard]),
            "a worn type outside the immune set grants nothing"
        );
    }

    /// An EMPTY immune set exempts nobody (the common no-protection field).
    #[test]
    fn an_empty_immune_set_exempts_nobody() {
        assert!(
            !exempts(ArmorType::Flak, &[]),
            "an empty immune set means the field drains everyone"
        );
    }
}

//! Field immunity by armor type.

use bevy::platform::collections::HashSet;
use serde::{Deserialize, Serialize, Serializer};

use super::{ApplyFieldEffect, DrainExempt, OccupantArmor};
use crate::armor::ArmorType;

/// Set of armor types immune to a field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ImmuneArmorTypes(HashSet<ArmorType>);

impl ImmuneArmorTypes {
    /// From an iterator of armor types.
    #[must_use]
    pub fn new(types: impl IntoIterator<Item = ArmorType>) -> Self {
        Self(types.into_iter().collect())
    }

    /// Whether a type is immune.
    #[must_use]
    pub fn contains(&self, armor_type: &ArmorType) -> bool {
        self.0.contains(armor_type)
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The listed types in [`ArmorType::ALL`] order, so a reader never sees hash order.
    pub fn iter(&self) -> impl Iterator<Item = ArmorType> + '_ {
        ArmorType::ALL
            .into_iter()
            .filter(|armor_type| self.0.contains(armor_type))
    }
}

/// Written in [`ArmorType::ALL`] order so the same set always writes the same bytes.
impl Serialize for ImmuneArmorTypes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

/// Exempts occupants wearing any listed armor type.
pub struct ApplyImmunity<'s> {
    armor_types: &'s ImmuneArmorTypes,
}

impl<'s> ApplyImmunity<'s> {
    /// Build the applicator.
    #[must_use]
    pub const fn new(armor_types: &'s ImmuneArmorTypes) -> Self {
        Self { armor_types }
    }
}

impl ApplyFieldEffect for ApplyImmunity<'_> {
    fn exempts_occupant(&self, armor: &OccupantArmor<'_, '_, '_>) -> DrainExempt {
        DrainExempt::new(armor.wears.pieces().any(|piece| {
            armor
                .worn
                .get(piece)
                .is_ok_and(|armor_type| self.armor_types.contains(armor_type))
        }))
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
        *ApplyImmunity::new(&set).exempts_occupant(&armor)
    }

    #[test]
    fn a_matching_worn_armor_type_exempts_the_occupant() {
        assert!(
            exempts(ArmorType::Flak, &[ArmorType::Flak, ArmorType::Hazard]),
            "ANY worn piece in the immune set grants whole-source immunity"
        );
    }

    #[test]
    fn a_non_matching_worn_armor_type_does_not_exempt() {
        assert!(
            !exempts(ArmorType::Plated, &[ArmorType::Flak, ArmorType::Hazard]),
            "a worn type outside the immune set grants nothing"
        );
    }

    #[test]
    fn an_empty_immune_set_exempts_nobody() {
        assert!(
            !exempts(ArmorType::Flak, &[]),
            "an empty immune set means the field drains everyone"
        );
    }
}

//! Ganger spawn builder for tests.

use super::registries::{TEST_ARMOR_KEY, TEST_WEAPON_KEY};
use crate::{
    armor::ArmorName,
    ganger::{
        Aim, Aiming, Cool, Direction, Facing, Faction, GangerName, Grit, LifeState, Luck, Reflexes,
        Speed, Stance, StanceKind, Strength, Toughness,
    },
    metric::CellLevel,
    situation::GangerSpawn,
    weapon::WeaponName,
};

/// Fluent builder for [`GangerSpawn`].
#[derive(Debug, Clone)]
pub struct GangerSpawnBuilder {
    spawn: GangerSpawn,
}

impl GangerSpawnBuilder {
    /// Defaults suitable for most tests.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spawn: GangerSpawn {
                at: super::registries::key(0, 0, 0),
                name: GangerName::new(String::from("Test Ganger")),
                faction: Faction::new(0),
                facing: Facing::new(Direction::East),
                stance: Stance::new(StanceKind::Standing),
                aiming: Aiming::new(true),
                life_state: LifeState::Alive,
                speed: Speed::new(3.0),
                aim: Aim::new(2.0),
                strength: Strength::new(3.0),
                toughness: Toughness::new(10.0),
                reflexes: Reflexes::new(2.0),
                cool: Cool::new(8.0),
                grit: Grit::new(18.0),
                luck: Luck::new(1.0),
                armor: ArmorName::new(TEST_ARMOR_KEY.to_owned()),
                weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            },
        }
    }

    /// Cell position.
    #[must_use]
    pub const fn at(mut self, at: CellLevel) -> Self {
        self.spawn.at = at;
        self
    }

    /// Display name.
    #[must_use]
    pub fn name(mut self, name: GangerName) -> Self {
        self.spawn.name = name;
        self
    }

    /// Faction id.
    #[must_use]
    pub const fn faction(mut self, faction: Faction) -> Self {
        self.spawn.faction = faction;
        self
    }

    /// Facing direction.
    #[must_use]
    pub const fn facing(mut self, facing: Facing) -> Self {
        self.spawn.facing = facing;
        self
    }

    /// Stance.
    #[must_use]
    pub const fn stance(mut self, stance: Stance) -> Self {
        self.spawn.stance = stance;
        self
    }

    /// Aiming flag.
    #[must_use]
    pub const fn aiming(mut self, aiming: Aiming) -> Self {
        self.spawn.aiming = aiming;
        self
    }

    /// Life state.
    #[must_use]
    pub const fn life_state(mut self, life_state: LifeState) -> Self {
        self.spawn.life_state = life_state;
        self
    }

    /// Speed attribute.
    #[must_use]
    pub const fn speed(mut self, speed: Speed) -> Self {
        self.spawn.speed = speed;
        self
    }

    /// Aim attribute.
    #[must_use]
    pub const fn aim(mut self, aim: Aim) -> Self {
        self.spawn.aim = aim;
        self
    }

    /// Strength attribute.
    #[must_use]
    pub const fn strength(mut self, strength: Strength) -> Self {
        self.spawn.strength = strength;
        self
    }

    /// Reflexes attribute.
    #[must_use]
    pub const fn reflexes(mut self, reflexes: Reflexes) -> Self {
        self.spawn.reflexes = reflexes;
        self
    }

    /// Cool attribute.
    #[must_use]
    pub const fn cool(mut self, cool: Cool) -> Self {
        self.spawn.cool = cool;
        self
    }

    /// Grit attribute.
    #[must_use]
    pub const fn grit(mut self, grit: Grit) -> Self {
        self.spawn.grit = grit;
        self
    }

    /// Toughness attribute.
    #[must_use]
    pub const fn toughness(mut self, toughness: Toughness) -> Self {
        self.spawn.toughness = toughness;
        self
    }

    /// Luck attribute.
    #[must_use]
    pub const fn luck(mut self, luck: Luck) -> Self {
        self.spawn.luck = luck;
        self
    }

    /// Armor key.
    #[must_use]
    pub fn armor(mut self, armor: ArmorName) -> Self {
        self.spawn.armor = armor;
        self
    }

    /// Weapon key.
    #[must_use]
    pub fn weapon(mut self, weapon: WeaponName) -> Self {
        self.spawn.weapon = weapon;
        self
    }

    /// Finish the spawn.
    #[must_use]
    pub fn build(self) -> GangerSpawn {
        self.spawn
    }
}

impl Default for GangerSpawnBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick spawn at a cell for the given faction index.
#[must_use]
pub fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .aim(Aim::new(f32::from(faction) + 2.0))
        .toughness(Toughness::new(f32::from(faction) + 3.0))
        .luck(Luck::new(f32::from(faction) + 1.0))
        .build()
}

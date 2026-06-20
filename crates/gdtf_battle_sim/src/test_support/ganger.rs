//! The fluent [`GangerSpawnBuilder`] — the canonical way every test constructs a
//! [`GangerSpawn`], with sane test defaults and per-field override methods — plus
//! the thin [`ganger_at`] free fn for the common `(at, faction)` case the old
//! per-module `ganger_at` helpers all reduced to.

use super::registries::{TEST_ARMOR_KEY, TEST_WEAPON_KEY};
use crate::{
    armor::ArmorName,
    ganger::{
        Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Shooting,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    metric::CellLevel,
    situation::GangerSpawn,
    weapon::WeaponName,
};

/// A fluent builder for a [`GangerSpawn`] with sane test defaults — the canonical
/// way every test (the sim's own unit tests and the downstream crates') builds an
/// authored ganger, replacing the hand-rolled 18-field struct literals.
///
/// The defaults are an Alive, faction-0, standing ganger with full vitals (HP 40,
/// Wounds 3, TU 60), facing East, aiming, referencing the test weapon + armor keys
/// ([`TEST_WEAPON_KEY`] / [`TEST_ARMOR_KEY`]) — so a default builder resolves
/// cleanly against the central
/// [`test_weapon_registry`](super::registries::test_weapon_registry) and
/// [`test_armor_registry`](super::registries::test_armor_registry). Override any
/// field with the corresponding method; [`build`](GangerSpawnBuilder::build) yields
/// the [`GangerSpawn`].
#[derive(Debug, Clone)]
pub struct GangerSpawnBuilder {
    spawn: GangerSpawn,
}

impl GangerSpawnBuilder {
    /// A fresh builder with the sane test defaults (faction 0, standing, Alive, HP
    /// 40 / Wounds 3 / TU 60, facing East, aiming, the test weapon + armor keys).
    /// Default place is `(0, 0, 0)`; set it with [`at`](GangerSpawnBuilder::at).
    #[must_use]
    pub fn new() -> Self {
        Self {
            spawn: GangerSpawn {
                at:         super::registries::key(0, 0, 0),
                name:       GangerName::new(String::from("Test Ganger")),
                faction:    Faction::new(0),
                facing:     Facing::new(Direction::East),
                stance:     Stance::new(StanceKind::Standing),
                aiming:     Aiming::new(true),
                hp:         Hp::new(40),
                hp_max:     HpMax::new(40),
                wounds:     Wounds::new(3),
                wounds_max: WoundsMax::new(3),
                tu:         Tu::new(60),
                tu_max:     TuMax::new(60),
                life_state: LifeState::Alive,
                shooting:   Shooting::new(2.0),
                toughness:  Toughness::new(3.0),
                luck:       Luck::new(1.0),
                armor:      ArmorName::new(TEST_ARMOR_KEY.to_owned()),
                weapon:     WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            },
        }
    }

    /// Place the ganger at `(cell, level)` — its spawned
    /// [`Position`](crate::ganger::Position).
    #[must_use]
    pub const fn at(mut self, at: CellLevel) -> Self {
        self.spawn.at = at;
        self
    }

    /// Set the ganger's display name.
    #[must_use]
    pub fn name(mut self, name: GangerName) -> Self {
        self.spawn.name = name;
        self
    }

    /// Set the ganger's gang (faction) identity.
    #[must_use]
    pub const fn faction(mut self, faction: Faction) -> Self {
        self.spawn.faction = faction;
        self
    }

    /// Set the ganger's facing.
    #[must_use]
    pub const fn facing(mut self, facing: Facing) -> Self {
        self.spawn.facing = facing;
        self
    }

    /// Set the ganger's stance (posture).
    #[must_use]
    pub const fn stance(mut self, stance: Stance) -> Self {
        self.spawn.stance = stance;
        self
    }

    /// Set the ganger's aim-mode flag.
    #[must_use]
    pub const fn aiming(mut self, aiming: Aiming) -> Self {
        self.spawn.aiming = aiming;
        self
    }

    /// Set the ganger's current hit-points pool.
    #[must_use]
    pub const fn hp(mut self, hp: Hp) -> Self {
        self.spawn.hp = hp;
        self
    }

    /// Set the ganger's HP maximum (the HP bar's denominator).
    #[must_use]
    pub const fn hp_max(mut self, hp_max: HpMax) -> Self {
        self.spawn.hp_max = hp_max;
        self
    }

    /// Set the ganger's current Wounds (life) pool.
    #[must_use]
    pub const fn wounds(mut self, wounds: Wounds) -> Self {
        self.spawn.wounds = wounds;
        self
    }

    /// Set the ganger's Wounds maximum (the Wounds pip count).
    #[must_use]
    pub const fn wounds_max(mut self, wounds_max: WoundsMax) -> Self {
        self.spawn.wounds_max = wounds_max;
        self
    }

    /// Set the ganger's current Time-Unit budget.
    #[must_use]
    pub const fn tu(mut self, tu: Tu) -> Self {
        self.spawn.tu = tu;
        self
    }

    /// Set the ganger's TU maximum (the round-start ceiling).
    #[must_use]
    pub const fn tu_max(mut self, tu_max: TuMax) -> Self {
        self.spawn.tu_max = tu_max;
        self
    }

    /// Set the ganger's terminal life state.
    #[must_use]
    pub const fn life_state(mut self, life_state: LifeState) -> Self {
        self.spawn.life_state = life_state;
        self
    }

    /// Set the ganger's Shooting combat stat.
    #[must_use]
    pub const fn shooting(mut self, shooting: Shooting) -> Self {
        self.spawn.shooting = shooting;
        self
    }

    /// Set the ganger's Toughness attribute.
    #[must_use]
    pub const fn toughness(mut self, toughness: Toughness) -> Self {
        self.spawn.toughness = toughness;
        self
    }

    /// Set the ganger's Luck attribute.
    #[must_use]
    pub const fn luck(mut self, luck: Luck) -> Self {
        self.spawn.luck = luck;
        self
    }

    /// Set the ganger's armor KEY (resolved against the armor registry at setup).
    #[must_use]
    pub fn armor(mut self, armor: ArmorName) -> Self {
        self.spawn.armor = armor;
        self
    }

    /// Set the ganger's weapon KEY (resolved against the weapon registry at setup).
    #[must_use]
    pub fn weapon(mut self, weapon: WeaponName) -> Self {
        self.spawn.weapon = weapon;
        self
    }

    /// Consume the builder and yield the configured [`GangerSpawn`].
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

/// Build an authored ganger at `at` with the given faction and otherwise
/// sane-but-DISTINCT-per-faction component values — the thin common-case helper
/// over [`GangerSpawnBuilder`] the old per-module `ganger_at` copies all reduced
/// to, so existing call sites migrate cleanly.
///
/// The name is `"Ganger {faction}"`, and the E3.0 attribute stats
/// ([`Shooting`] / [`Toughness`] / [`Luck`]) are offset by the faction so a
/// per-field readback is provable per faction (NOT shipped tuning; per-ganger
/// data). Every other field is the builder default.
#[must_use]
pub fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .shooting(Shooting::new(f32::from(faction) + 2.0))
        .toughness(Toughness::new(f32::from(faction) + 3.0))
        .luck(Luck::new(f32::from(faction) + 1.0))
        .build()
}

//! The fluent [`GangerSpawnBuilder`] — the canonical way every test constructs a
//! [`GangerSpawn`], with sane test defaults and per-field override methods — plus
//! the thin [`ganger_at`] free fn for the common `(at, faction)` case the old
//! per-module `ganger_at` helpers all reduced to.

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

/// A fluent builder for a [`GangerSpawn`] with sane test defaults — the canonical
/// way every test (the sim's own unit tests and the downstream crates') builds an
/// authored ganger, replacing the hand-rolled struct literals.
///
/// Since GTW-384 the builder authors the EIGHT DIRECT ATTRIBUTES (not the computed
/// stats — those are derived at setup). The defaults are an Alive, faction-0, standing
/// ganger, facing East, aiming, with attributes (Speed 3 / Aim 2 / Strength 3 /
/// Toughness 10 / Reflexes 2 / Cool 8 / Grit 18 / Luck 1) authored so the
/// DEFAULT-weight derivation lands in the pre-GTW-384 ballpark (HP ~32, Wounds 3,
/// TU 60). These are ARBITRARY test DATA, never pinned by a test (the derivation tests
/// assert the RELATION attributes → stats, not a magnitude). References the test weapon
/// and armor keys ([`TEST_WEAPON_KEY`] / [`TEST_ARMOR_KEY`]) so a default builder resolves
/// cleanly against the central
/// [`test_weapon_registry`](super::registries::test_weapon_registry) and
/// [`test_armor_registry`](super::registries::test_armor_registry). Override any field
/// with the corresponding method; [`build`](GangerSpawnBuilder::build) yields the
/// [`GangerSpawn`].
#[derive(Debug, Clone)]
pub struct GangerSpawnBuilder {
    spawn: GangerSpawn,
}

impl GangerSpawnBuilder {
    /// A fresh builder with the sane test defaults (faction 0, standing, Alive, facing
    /// East, aiming, the eight attributes authored to the ballpark above, the test
    /// weapon + armor keys). Default place is `(0, 0, 0)`; set it with
    /// [`at`](GangerSpawnBuilder::at).
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
                life_state: LifeState::Alive,
                speed:      Speed::new(3.0),
                aim:        Aim::new(2.0),
                strength:   Strength::new(3.0),
                toughness:  Toughness::new(10.0),
                reflexes:   Reflexes::new(2.0),
                cool:       Cool::new(8.0),
                grit:       Grit::new(18.0),
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

    /// Set the ganger's terminal life state.
    #[must_use]
    pub const fn life_state(mut self, life_state: LifeState) -> Self {
        self.spawn.life_state = life_state;
        self
    }

    /// Set the ganger's **Speed** direct attribute (GTW-384) — drives the derived TU
    /// budget + Fight/Reactions terms.
    #[must_use]
    pub const fn speed(mut self, speed: Speed) -> Self {
        self.spawn.speed = speed;
        self
    }

    /// Set the ganger's **Aim** direct attribute (GTW-384) — the dominant derived
    /// Shooting term.
    #[must_use]
    pub const fn aim(mut self, aim: Aim) -> Self {
        self.spawn.aim = aim;
        self
    }

    /// Set the ganger's **Strength** direct attribute (GTW-384) — a derived Fight term.
    #[must_use]
    pub const fn strength(mut self, strength: Strength) -> Self {
        self.spawn.strength = strength;
        self
    }

    /// Set the ganger's **Reflexes** direct attribute (GTW-384) — a derived
    /// Shooting + Reactions term.
    #[must_use]
    pub const fn reflexes(mut self, reflexes: Reflexes) -> Self {
        self.spawn.reflexes = reflexes;
        self
    }

    /// Set the ganger's **Cool** direct attribute (GTW-384) — the broad
    /// Shooting/Fight/Reactions/HP/Morale contributor.
    #[must_use]
    pub const fn cool(mut self, cool: Cool) -> Self {
        self.spawn.cool = cool;
        self
    }

    /// Set the ganger's **Grit** direct attribute (GTW-384) — the dominant derived
    /// HP + Morale term.
    #[must_use]
    pub const fn grit(mut self, grit: Grit) -> Self {
        self.spawn.grit = grit;
        self
    }

    /// Set the ganger's **Toughness** direct attribute — the (reused) severity-roll
    /// mitigation, also a derived HP term (GTW-384).
    #[must_use]
    pub const fn toughness(mut self, toughness: Toughness) -> Self {
        self.spawn.toughness = toughness;
        self
    }

    /// Set the ganger's **Luck** direct attribute — the (reused) severity-roll tail
    /// modulator (feeds the severity roll only, never the computed stats).
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
/// sane-but-DISTINCT-per-faction attribute values — the thin common-case helper
/// over [`GangerSpawnBuilder`] the old per-module `ganger_at` copies all reduced
/// to, so existing call sites migrate cleanly.
///
/// The name is `"Ganger {faction}"`, and three direct attributes
/// ([`Aim`] / [`Toughness`] / [`Luck`]) are offset by the faction so a per-attribute
/// readback (and the per-faction derived-stat RELATION) is provable per faction (NOT
/// shipped tuning; per-ganger data). Every other field is the builder default.
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

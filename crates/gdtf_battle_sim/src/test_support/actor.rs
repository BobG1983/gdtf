//! Helpers for spawning ganger entities in tests.

use bevy::prelude::{Bundle, Entity, World};

use crate::{
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    metric::CellLevel,
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WieldedBy},
};

/// Single-shot fire mode with the given TU percent and shot count.
#[must_use]
pub const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// Minimal alive target with the given HP and wounds.
#[must_use]
pub fn target_bundle(hp: u16, wounds: u8) -> impl Bundle {
    (
        Hp::new(hp),
        Wounds::new(wounds),
        LifeState::Alive,
        InflictedWounds::default(),
        Toughness::new(1.0),
        Luck::new(0.0),
    )
}

/// Spawn a weapon entity wielded by `ganger`.
pub fn wield(world: &mut World, ganger: Entity, weapon: impl Bundle) -> Entity {
    world.spawn((WieldedBy::new(ganger), weapon)).id()
}

/// Fluent builder for a ganger entity in a test world.
#[derive(Debug, Clone, Default)]
pub struct GangerEntityBuilder {
    at: Option<CellLevel>,
    faction: Option<Faction>,
    stance: Option<StanceKind>,
    facing: Option<Direction>,
    aiming: Option<bool>,
    life_state: Option<LifeState>,
    tu: Option<u8>,
    tu_max: Option<u8>,
    hp: Option<u16>,
    wounds: Option<u8>,
    inflicted_wounds: bool,
    toughness: Option<f32>,
    luck: Option<f32>,
    shooting: Option<f32>,
}

impl GangerEntityBuilder {
    /// Empty builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            at: None,
            faction: None,
            stance: None,
            facing: None,
            aiming: None,
            life_state: None,
            tu: None,
            tu_max: None,
            hp: None,
            wounds: None,
            inflicted_wounds: false,
            toughness: None,
            luck: None,
            shooting: None,
        }
    }

    /// Position.
    #[must_use]
    pub const fn at(mut self, at: CellLevel) -> Self {
        self.at = Some(at);
        self
    }

    /// Faction.
    #[must_use]
    pub const fn faction(mut self, faction: Faction) -> Self {
        self.faction = Some(faction);
        self
    }

    /// Stance kind.
    #[must_use]
    pub const fn stance(mut self, stance: StanceKind) -> Self {
        self.stance = Some(stance);
        self
    }

    /// Facing direction.
    #[must_use]
    pub const fn facing(mut self, facing: Direction) -> Self {
        self.facing = Some(facing);
        self
    }

    /// Aiming flag.
    #[must_use]
    pub const fn aiming(mut self, aiming: bool) -> Self {
        self.aiming = Some(aiming);
        self
    }

    /// Life state.
    #[must_use]
    pub const fn life_state(mut self, life_state: LifeState) -> Self {
        self.life_state = Some(life_state);
        self
    }

    /// Current TU.
    #[must_use]
    pub const fn tu(mut self, tu: u8) -> Self {
        self.tu = Some(tu);
        self
    }

    /// Max TU.
    #[must_use]
    pub const fn tu_max(mut self, tu_max: u8) -> Self {
        self.tu_max = Some(tu_max);
        self
    }

    /// Hit points.
    #[must_use]
    pub const fn hp(mut self, hp: u16) -> Self {
        self.hp = Some(hp);
        self
    }

    /// Wound count.
    #[must_use]
    pub const fn wounds(mut self, wounds: u8) -> Self {
        self.wounds = Some(wounds);
        self
    }

    /// Toughness.
    #[must_use]
    pub const fn toughness(mut self, toughness: f32) -> Self {
        self.toughness = Some(toughness);
        self
    }

    /// Luck.
    #[must_use]
    pub const fn luck(mut self, luck: f32) -> Self {
        self.luck = Some(luck);
        self
    }

    /// Shooting skill.
    #[must_use]
    pub const fn shooting(mut self, shooting: f32) -> Self {
        self.shooting = Some(shooting);
        self
    }

    /// Set HP, wounds, alive, empty wound list, and baseline toughness/luck.
    #[must_use]
    pub const fn combat_vitals(mut self, hp: u16, wounds: u8) -> Self {
        self.hp = Some(hp);
        self.wounds = Some(wounds);
        self.life_state = Some(LifeState::Alive);
        self.inflicted_wounds = true;
        self.toughness = Some(1.0);
        self.luck = Some(0.0);
        self
    }

    /// Spawn into the world and return the entity id.
    #[must_use]
    pub fn spawn(self, world: &mut World) -> Entity {
        let mut entity = world.spawn(());
        if let Some(at) = self.at {
            entity.insert(Position::new(at));
        }
        if let Some(faction) = self.faction {
            entity.insert(faction);
        }
        if let Some(stance) = self.stance {
            entity.insert(Stance::new(stance));
        }
        if let Some(facing) = self.facing {
            entity.insert(Facing::new(facing));
        }
        if let Some(aiming) = self.aiming {
            entity.insert(Aiming::new(aiming));
        }
        if let Some(life_state) = self.life_state {
            entity.insert(life_state);
        }
        if let Some(tu) = self.tu {
            entity.insert(Tu::new(tu));
        }
        if let Some(tu_max) = self.tu_max {
            entity.insert(TuMax::new(tu_max));
        }
        if let Some(hp) = self.hp {
            entity.insert(Hp::new(hp));
        }
        if let Some(wounds) = self.wounds {
            entity.insert(Wounds::new(wounds));
        }
        if self.inflicted_wounds {
            entity.insert(InflictedWounds::default());
        }
        if let Some(toughness) = self.toughness {
            entity.insert(Toughness::new(toughness));
        }
        if let Some(luck) = self.luck {
            entity.insert(Luck::new(luck));
        }
        if let Some(shooting) = self.shooting {
            entity.insert(Shooting::new(shooting));
        }
        entity.id()
    }
}

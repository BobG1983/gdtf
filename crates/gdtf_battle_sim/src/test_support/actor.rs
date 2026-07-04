//! The knobbed ganger/target ENTITY spawner + the canonical per-mode/bundle
//! fixtures (GTW-576) — the shared halves the per-file `spawn_ganger` /
//! `target_bundle` / `single_mode` copies collapsed onto.
//!
//! [`GangerEntityBuilder`] is opt-in per component: ONLY the knobs a test sets
//! insert components, so a selection test spawns `(Faction, Position)` and nothing
//! else, while a fire test spawns the full combat-vitals set — the exact component
//! sets the former hand-rolled helpers spawned, never a fixed superset (a query
//! with `Without<..>`/entity-count semantics sees the same world as before).

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

/// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers — the
/// canonical [`ModeKind::Single`] fixture every suite builds its test weapon's mode
/// from. A test pinning a different kind/cone builds a [`FireModeSpec`] directly.
#[must_use]
pub const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// The per-ganger battle-state bundle a fire target carries (the target query set)
/// — arbitrary magnitudes. Since GTW-323 (ADR-0004) the combat armor lives on
/// related piece entities, NOT the ganger, so this bundle carries no armor.
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

/// Spawn + relate a wielded-weapon entity carrying `weapon` on `ganger` — the
/// `World`-test equivalent of `setup_battle`'s `queue_spawn_related_scenes::<Wields>`.
/// The [`WieldedBy`] insert hook populates the ganger's
/// [`Wields`](crate::weapon::Wields) collection **synchronously** in a bare `World`
/// spawn, so the very next dispatch resolves it. `weapon` is whatever component set
/// the suite needs on the weapon entity (a full `WeaponBundle`, or just a
/// `FireMode` selector + `Magazine` + `Handedness`).
pub fn wield(world: &mut World, ganger: Entity, weapon: impl Bundle) -> Entity {
    world.spawn((WieldedBy::new(ganger), weapon)).id()
}

/// The opt-in knobbed ganger-entity spawner — the canonical replacement for the
/// per-file `spawn_ganger` helpers. Every knob is `Option`al and OFF by default:
/// [`spawn`](Self::spawn) inserts EXACTLY the components whose knobs were set, so
/// each migrated call site spawns the same component set it hand-rolled before.
///
/// [`combat_vitals`](Self::combat_vitals) is the [`target_bundle`] group as knobs
/// (Hp / Wounds / `LifeState::Alive` / [`InflictedWounds`] / Toughness `1.0` / Luck
/// `0.0`); setting an individual knob afterwards overrides just that field. Exotic
/// per-suite components (a `PeekOffset`, an `OnDeath`) ride a call-site
/// `world.entity_mut(e).insert(..)` after the spawn — the builder stays the common
/// ganger vocabulary, never a mode-flagged union of every suite's extras.
#[derive(Debug, Clone, Default)]
pub struct GangerEntityBuilder {
    /// `Position` — the ganger's `(cell, level)`.
    at:               Option<CellLevel>,
    /// `Faction` — the ganger's gang identity.
    faction:          Option<Faction>,
    /// `Stance` — the ganger's posture.
    stance:           Option<StanceKind>,
    /// `Facing` — the ganger's facing direction.
    facing:           Option<Direction>,
    /// `Aiming` — the aim-mode flag.
    aiming:           Option<bool>,
    /// `LifeState` — the terminal life state.
    life_state:       Option<LifeState>,
    /// `Tu` — the current time-unit pool.
    tu:               Option<u8>,
    /// `TuMax` — the per-turn time-unit budget.
    tu_max:           Option<u8>,
    /// `Hp` — the current hit points.
    hp:               Option<u16>,
    /// `Wounds` — the current wound pool.
    wounds:           Option<u8>,
    /// `InflictedWounds` — the empty inflicted-wound ledger (GTW-279).
    inflicted_wounds: bool,
    /// `Toughness` — the severity-roll mitigation attribute.
    toughness:        Option<f32>,
    /// `Luck` — the severity-roll tail modulator.
    luck:             Option<f32>,
    /// `Shooting` — the derived shooting stat.
    shooting:         Option<f32>,
}

impl GangerEntityBuilder {
    /// A fresh builder with every knob off — [`spawn`](Self::spawn) on it inserts
    /// nothing but the entity.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            at:               None,
            faction:          None,
            stance:           None,
            facing:           None,
            aiming:           None,
            life_state:       None,
            tu:               None,
            tu_max:           None,
            hp:               None,
            wounds:           None,
            inflicted_wounds: false,
            toughness:        None,
            luck:             None,
            shooting:         None,
        }
    }

    /// Place the ganger at `(cell, level)` — inserts `Position`.
    #[must_use]
    pub const fn at(mut self, at: CellLevel) -> Self {
        self.at = Some(at);
        self
    }

    /// Set the ganger's gang — inserts `Faction`.
    #[must_use]
    pub const fn faction(mut self, faction: Faction) -> Self {
        self.faction = Some(faction);
        self
    }

    /// Set the ganger's posture — inserts `Stance`.
    #[must_use]
    pub const fn stance(mut self, stance: StanceKind) -> Self {
        self.stance = Some(stance);
        self
    }

    /// Set the ganger's facing — inserts `Facing`.
    #[must_use]
    pub const fn facing(mut self, facing: Direction) -> Self {
        self.facing = Some(facing);
        self
    }

    /// Set the aim-mode flag — inserts `Aiming`.
    #[must_use]
    pub const fn aiming(mut self, aiming: bool) -> Self {
        self.aiming = Some(aiming);
        self
    }

    /// Set the terminal life state — inserts `LifeState` (overrides the `Alive` a
    /// prior [`combat_vitals`](Self::combat_vitals) set).
    #[must_use]
    pub const fn life_state(mut self, life_state: LifeState) -> Self {
        self.life_state = Some(life_state);
        self
    }

    /// Set the current TU pool — inserts `Tu`.
    #[must_use]
    pub const fn tu(mut self, tu: u8) -> Self {
        self.tu = Some(tu);
        self
    }

    /// Set the per-turn TU budget — inserts `TuMax`.
    #[must_use]
    pub const fn tu_max(mut self, tu_max: u8) -> Self {
        self.tu_max = Some(tu_max);
        self
    }

    /// Set the current hit points — inserts `Hp`.
    #[must_use]
    pub const fn hp(mut self, hp: u16) -> Self {
        self.hp = Some(hp);
        self
    }

    /// Set the current wound pool — inserts `Wounds`.
    #[must_use]
    pub const fn wounds(mut self, wounds: u8) -> Self {
        self.wounds = Some(wounds);
        self
    }

    /// Set the severity-roll mitigation — inserts `Toughness`.
    #[must_use]
    pub const fn toughness(mut self, toughness: f32) -> Self {
        self.toughness = Some(toughness);
        self
    }

    /// Set the severity-roll tail modulator — inserts `Luck`.
    #[must_use]
    pub const fn luck(mut self, luck: f32) -> Self {
        self.luck = Some(luck);
        self
    }

    /// Set the derived shooting stat — inserts `Shooting`.
    #[must_use]
    pub const fn shooting(mut self, shooting: f32) -> Self {
        self.shooting = Some(shooting);
        self
    }

    /// The [`target_bundle`] group as knobs: `Hp(hp)` + `Wounds(wounds)` +
    /// `LifeState::Alive` + [`InflictedWounds::default`] + `Toughness(1.0)` +
    /// `Luck(0.0)` — the full per-ganger battle-state set the fire target query
    /// reads. Set an individual knob AFTER this to override just that field.
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

    /// Spawn the entity carrying EXACTLY the knob-selected components; returns it.
    /// A bare-`World` spawn in a TEST BODY context (the `bevy-traps.md` #7 headless
    /// carve-out — the established `test_support` spawner idiom).
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

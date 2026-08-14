//! Worlds one `try_fire_request` call is made against, and the call itself.

use bevy::{
    ecs::system::SystemState,
    prelude::{Entity, World},
};
use gdtf_battle_input::{ShooterArms, ShotRefusal, try_fire_request};
use gdtf_battle_sim::{
    acts::FireRequested,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::GRID_WIDTH,
    prelude::LifeState,
    test_support::{GangerEntityBuilder, single_mode, wield},
    tuning::CombatTuning,
    weapon::{FireModeSpec, Handedness, MagazineSize},
};

/// The mode every case prices its shot against: a fifth of the pool, one shot.
const PROBE_MODE: FireModeSpec = single_mode(0.2, 1);

/// A target cell well inside the battle grid.
pub(crate) fn on_grid() -> CellLevel {
    CellLevel::new(Cell::new(10, 10), Level::new(0))
}

/// A target cell one column past the right edge of the battle grid.
pub(crate) fn off_grid() -> CellLevel {
    CellLevel::new(
        Cell::new(i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX), 10),
        Level::new(0),
    )
}

/// A world holding one candidate shooter, ready for a fire request.
pub(crate) struct FireCase {
    world:   World,
    shooter: Entity,
}

impl FireCase {
    /// An entity carrying none of the parts a shooter needs.
    pub(crate) fn not_a_shooter() -> Self {
        let mut world = World::new();
        let shooter = world.spawn_empty().id();
        Self { world, shooter }
    }

    /// A shooter carrying every firing part, wielding nothing.
    pub(crate) fn unarmed_shooter() -> Self {
        let mut world = World::new();
        let shooter = spawn_shooter(&mut world);
        Self { world, shooter }
    }

    /// A shooter wielding a one-handed weapon whose magazine holds `rounds`.
    pub(crate) fn armed_shooter(rounds: u16) -> Self {
        let mut world = World::new();
        let shooter = spawn_shooter(&mut world);
        wield(
            &mut world,
            shooter,
            (
                Magazine::new(
                    LoadedRounds::new(rounds),
                    MagazineSize::new(30),
                    ReloadTu::new(12),
                ),
                Handedness::OneHanded,
            ),
        );
        Self { world, shooter }
    }

    /// What `try_fire_request` answers for this shooter at `target`.
    ///
    /// `None` when the shooter's arms could not be read out of the world at all.
    pub(crate) fn request(
        &mut self,
        target: CellLevel,
    ) -> Option<Result<FireRequested, ShotRefusal>> {
        let mut state: SystemState<ShooterArms<'static, 'static>> =
            SystemState::new(&mut self.world);
        let arms = state.get(&self.world).ok()?;
        Some(try_fire_request(
            self.shooter,
            target,
            PROBE_MODE,
            &CombatTuning::default(),
            &arms,
        ))
    }
}

// A living shooter hip-firing with a pool that covers the probe mode several times over.
fn spawn_shooter(world: &mut World) -> Entity {
    GangerEntityBuilder::new()
        .life_state(LifeState::Alive)
        .tu(200)
        .tu_max(100)
        .aiming(false)
        .spawn(world)
}

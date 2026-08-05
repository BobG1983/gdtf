//! System params for the reaction pass: triggers, reactor arms, eligibility, grids.

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::{Changed, Deref, Entity, MessageReader, Query, Res, With},
};

use crate::{
    acts::FireDeclaration,
    cover::CoverLedger,
    ganger::{Position, Suppressed},
    magazine::Magazine,
    march::MarchGrids,
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ReactionsUsed},
    weapon::{FireMode, FireModeSpec, FiringWeapon, Handedness, ShotSilenced},
};

/// What could be reacted to this tick: gangers that moved and shots that were heard.
#[derive(SystemParam)]
pub struct ReactionTriggers<'w, 's> {
    moved:        Query<'w, 's, Entity, Changed<Position>>,
    declarations: MessageReader<'w, 's, FireDeclaration>,
}

impl ReactionTriggers<'_, '_> {
    /// Gangers that moved, plus shooters whose weapon was not silenced.
    pub(super) fn actors(&mut self, arms: &ReactorArms) -> HashSet<Entity> {
        let mut actors: HashSet<Entity> = self.moved.iter().collect();
        for declaration in self.declarations.read() {
            if *arms.silenced(declaration.shooter) {
                continue;
            }
            actors.insert(declaration.shooter);
        }
        actors
    }
}

/// The weapon a reactor swings on to: which one, how loud, and what it holds.
#[derive(SystemParam)]
pub struct ReactorArms<'w, 's> {
    firing:    FiringWeapon<'w, 's>,
    magazines: Query<'w, 's, (&'static Magazine, &'static FireMode, &'static Handedness)>,
}

/// One reactor's firing weapon, resolved for a single opportunity shot.
pub(super) struct ReactorShot {
    pub(super) mode:       FireModeSpec,
    pub(super) magazine:   Magazine,
    pub(super) handedness: Handedness,
    pub(super) weapon:     Entity,
}

impl ReactorArms<'_, '_> {
    /// Resolve this reactor's firing weapon to a single shot.
    pub(super) fn shot(&self, reactor: Entity) -> Option<ReactorShot> {
        let weapon = self.firing.of(reactor)?;
        let (magazine, fire_mode, handedness) = self.magazines.get(weapon).ok()?;
        Some(ReactorShot {
            mode: fire_mode.single(),
            magazine: *magazine,
            handedness: *handedness,
            weapon,
        })
    }

    /// Whether this shooter's weapon is silenced.
    pub(super) fn silenced(&self, shooter: Entity) -> ShotSilenced {
        self.firing.silenced(shooter)
    }
}

/// Whether a reactor is pinned down and cannot take an opportunity shot.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactorSuppressed(bool);

impl ReactorSuppressed {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(suppressed: bool) -> Self {
        Self(suppressed)
    }
}

/// What stops a reactor from taking another opportunity shot this round.
#[derive(SystemParam)]
pub struct ReactorEligibility<'w, 's> {
    suppressed: Query<'w, 's, (), With<Suppressed>>,
    used:       Query<'w, 's, &'static mut ReactionsUsed>,
}

impl ReactorEligibility<'_, '_> {
    /// A suppressed reactor never fires.
    pub(super) fn suppressed(&self, reactor: Entity) -> ReactorSuppressed {
        ReactorSuppressed::new(self.suppressed.get(reactor).is_ok())
    }

    /// Reactions this reactor has already spent, zero when it tracks none.
    pub(super) fn used_by(&self, reactor: Entity) -> ReactionsUsed {
        self.used
            .get(reactor)
            .copied()
            .unwrap_or_else(|_| ReactionsUsed::new(0))
    }

    /// Count one more reaction against this reactor.
    pub(super) fn record_use(&mut self, reactor: Entity) {
        if let Ok(mut counter) = self.used.get_mut(reactor) {
            counter.increment();
        }
    }
}

/// The grids and tuning an opportunity shot is judged against.
#[derive(SystemParam)]
pub struct ReactionGrids<'w> {
    occupancy: Res<'w, OccupancyGrid>,
    surface:   Res<'w, SurfaceGrid>,
    cover:     Res<'w, CoverLedger>,
    tuning:    Res<'w, CombatTuning>,
}

impl ReactionGrids<'_> {
    /// Grids a line of sight marches through.
    pub(super) fn march(&self) -> MarchGrids<'_> {
        MarchGrids {
            occupancy: &self.occupancy,
            surface:   &self.surface,
            cover:     &self.cover,
        }
    }

    /// Occupancy, for stair eye offsets.
    pub(super) fn occupancy(&self) -> &OccupancyGrid {
        &self.occupancy
    }

    /// Combat tuning.
    pub(super) fn tuning(&self) -> &CombatTuning {
        &self.tuning
    }
}

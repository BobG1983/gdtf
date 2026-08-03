use bevy::{
    math::Vec2,
    prelude::{Component, Deref, Entity},
};

use crate::{
    central_axis::{clamp_within_cell, muzzle_height, target_aim_point},
    cover::CoverLedger,
    ganger::{Facing, Position, Stance, StanceKind},
    march::{MarchDir, MarchKind, march_vector},
    metric::{CellLevel, SimPos, SimUnit, cell_center},
    occupancy::{OccupancyGrid, StairEyeOffset},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct PeekOffset(Vec2);

impl PeekOffset {
            #[must_use]
    pub const fn new(displacement: Vec2) -> Self {
        Self(displacement)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Observer<'a> {
        pub position:         &'a Position,
            pub stance:           &'a Stance,
            pub facing:           &'a Facing,
                        pub stair_eye_offset: StairEyeOffset,
                pub peek_offset:      PeekOffset,
}

#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
        pub position: &'a Position,
                pub stance:   &'a Stance,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sighted(bool);

impl Sighted {
        #[must_use]
    pub const fn new(sighted: bool) -> Self {
        Self(sighted)
    }
}

#[must_use]
pub fn has_los(
    from: &Observer,
    to: &Target,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(Entity) -> bool,
) -> Sighted {
    let observer_cell = cell_level_of(from.position);
    let target_cell = cell_level_of(to.position);

    if observer_cell == target_cell {
        return Sighted::new(true);
    }

    let eye = eye_anchor(from, tuning);
    let aim = aim_anchor(to, occupancy, cover, tuning);

    let dir = MarchDir::new((*aim - *eye).normalize_or_zero());

    let result = march_vector(
        eye,
        dir,
        occupancy,
        surface,
        cover,
        tuning,
        observer_cell,
        is_dead,
    );

    is_clear(&result, target_cell, eye, aim)
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "has_los_peeking forwards every argument has_los needs plus the explicit \
              peek displacement — the same arity justification as has_los and can_see; \
              bundling into a struct would only hide the arity without reducing it"
)]
pub fn has_los_peeking(
    from: &Observer,
    to: &Target,
    peek: PeekOffset,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(Entity) -> bool,
) -> Sighted {
    let peeking = Observer {
        peek_offset: peek,
        ..*from
    };
    has_los(&peeking, to, occupancy, surface, cover, tuning, is_dead)
}

pub(super) fn eye_anchor(observer: &Observer, tuning: &CombatTuning) -> SimPos {
    let (cell, level) = observer.position.split();
    let center = cell_center(cell, level);
    let base_z = f32::from(*level) + *muzzle_height(**observer.stance, tuning);
    let stair_z = match **observer.stance {
        StanceKind::Prone => 0.0,
        StanceKind::Standing | StanceKind::Crouching => *observer.stair_eye_offset,
    };
    let peek = *observer.peek_offset;
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer \
                  corner is exact for this range"
    )]
    let eye_x = *clamp_within_cell(SimUnit::new(center.x + peek.x), SimUnit::new(cell.x as f32));
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer \
                  corner is exact for this range"
    )]
    let eye_y = *clamp_within_cell(SimUnit::new(center.y + peek.y), SimUnit::new(cell.y as f32));
    SimPos::new(eye_x, eye_y, base_z + stair_z)
}

pub(super) fn aim_anchor(
    target: &Target,
    occupancy: &OccupancyGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> SimPos {
    let at = cell_level_of(target.position);
    let aim_band = cover
        .peek(&at)
        .map(|entry| entry.height_band)
        .or_else(|| occupancy.occupant_band(&at));
    target_aim_point(*target.position, *target.stance, aim_band, tuning)
}

fn cell_level_of(position: &Position) -> CellLevel {
    **position
}

fn is_clear(
    result: &crate::march::MarchResult,
    target_cell: CellLevel,
    eye: SimPos,
    aim: SimPos,
) -> Sighted {
    match result.kind {
        MarchKind::Miss | MarchKind::Ground => Sighted::new(true),
        MarchKind::Slab | MarchKind::Cover(_) | MarchKind::Ganger(_) => {
            if result.at == target_cell {
                return Sighted::new(true);
            }
            let impact_d2 = (*result.impact - *eye).length_squared();
            let aim_d2 = (*aim - *eye).length_squared();
            Sighted::new(impact_d2 >= aim_d2)
        }
    }
}

pub(super) use super::super::support::*;
pub(super) use crate::{
    acts::{ExecuteDownedRequested, StabilizeDownedRequested, execute_tu_cost, stabilize_tu_cost},
    effects::bleed::BleedingOut,
    tuning::CombatTuning,
};

// The frame each request was drained on, plus the steps the turn asked for.
pub(super) struct DownedDrive {
    pub(super) executes:   Vec<(usize, ExecuteDownedRequested)>,
    pub(super) stabilizes: Vec<(usize, StabilizeDownedRequested)>,
    pub(super) aims:       Vec<(usize, SetAimingRequested)>,
    pub(super) fires:      Vec<(usize, FireRequested)>,
    pub(super) moves:      Vec<MoveRequested>,
}

impl DownedDrive {
    // Frames an execute for this pair was drained on.
    pub(super) fn executes_for(&self, actor: Entity, target: Entity) -> Vec<usize> {
        self.executes
            .iter()
            .filter(|(_, request)| request.actor == actor && request.target == target)
            .map(|(frame, _)| *frame)
            .collect()
    }

    // Frames a stabilize for this pair was drained on.
    pub(super) fn stabilizes_for(&self, actor: Entity, target: Entity) -> Vec<usize> {
        self.stabilizes
            .iter()
            .filter(|(_, request)| request.actor == actor && request.target == target)
            .map(|(frame, _)| *frame)
            .collect()
    }

    // Earliest frame this actor's execute was drained on.
    pub(super) fn first_execute(&self, actor: Entity) -> Option<usize> {
        self.executes
            .iter()
            .filter(|(_, request)| request.actor == actor)
            .map(|(frame, _)| *frame)
            .min()
    }

    // Earliest frame this actor's aim-on was drained on.
    pub(super) fn first_aim(&self, actor: Entity) -> Option<usize> {
        self.aims
            .iter()
            .filter(|(_, request)| request.actor == actor)
            .map(|(frame, _)| *frame)
            .min()
    }

    // Earliest frame this actor's shot was drained on.
    pub(super) fn first_fire(&self, actor: Entity) -> Option<usize> {
        self.fires
            .iter()
            .filter(|(_, request)| request.shooter == actor)
            .map(|(frame, _)| *frame)
            .min()
    }
}

pub(super) fn execute_cost(app: &App) -> u8 {
    *execute_tu_cost(app.world().resource::<CombatTuning>())
}

pub(super) fn stabilize_cost(app: &App) -> u8 {
    *stabilize_tu_cost(app.world().resource::<CombatTuning>())
}

fn drain_executes(app: &mut App) -> Vec<ExecuteDownedRequested> {
    app.world_mut()
        .resource_mut::<Messages<ExecuteDownedRequested>>()
        .drain()
        .collect()
}

fn drain_stabilizes(app: &mut App) -> Vec<StabilizeDownedRequested> {
    app.world_mut()
        .resource_mut::<Messages<StabilizeDownedRequested>>()
        .drain()
        .collect()
}

fn lay_down(world: &mut World, ganger: Entity) {
    if let Some(mut life) = world.get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
}

pub(super) fn life_of(app: &App, ganger: Entity) -> Option<LifeState> {
    app.world().get::<LifeState>(ganger).copied()
}

pub(super) fn is_bleeding(app: &App, ganger: Entity) -> bool {
    app.world().get::<BleedingOut>(ganger).is_some()
}

// Down these gangers on the player's turn, so `mark_downed_bleeding` marks them first.
pub(super) fn down_before_the_enemy_turn(app: &mut App, gangers: &[Entity]) {
    app.insert_resource(ActiveFaction::new(PLAYER));
    for &ganger in gangers {
        lay_down(app.world_mut(), ganger);
    }
    app.update();
    app.insert_resource(ActiveFaction::new(ENEMY));
}

pub(super) fn drive_until_player(app: &mut App) -> DownedDrive {
    let mut drive = DownedDrive {
        executes:   Vec::new(),
        stabilizes: Vec::new(),
        aims:       Vec::new(),
        fires:      Vec::new(),
        moves:      Vec::new(),
    };
    let mut frame = 0_usize;
    drive_until_player_turn(app, |app| {
        drive.executes.extend(
            drain_executes(app)
                .into_iter()
                .map(|request| (frame, request)),
        );
        drive.stabilizes.extend(
            drain_stabilizes(app)
                .into_iter()
                .map(|request| (frame, request)),
        );
        drive
            .aims
            .extend(drain_aims(app).into_iter().map(|request| (frame, request)));
        drive
            .fires
            .extend(drain_fires(app).into_iter().map(|request| (frame, request)));
        drive.moves.extend(drain_moves(app));
        frame += 1;
    });
    drive
}

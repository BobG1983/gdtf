use bevy::prelude::*;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::battle_running::{resources::BattleRunningComplete, systems::*},
};

pub(in crate::states) struct GameBattleScapeBattleRunningScenePlugin;

impl Plugin for GameBattleScapeBattleRunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::BattleRunning), print_on_enter)
        // The battlescape now PERSISTS in `BattleRunning` (GTW-236): the old 3-tick
        // turn-budget auto-exit is gone. `move_on` advances `BattleRunning → AnimateOut`
        // ONLY once the explicit end-signal marker `BattleRunningComplete` is present — the
        // victory census / flee button (sibling slices) are what insert it. Until one of
        // those lands the marker is never inserted, so the battle simply rests here.
        //
        // `end_battle_on_outcome` (GTW-239) is the SECOND inserter of that marker (the
        // victory census's outcome twin of `gate_generation_complete`): it reads the
        // sim-owned `BattleWon` / `BattleLost` outcome buffers and inserts
        // `BattleRunningComplete` on EITHER. It runs in `Update` (the outcome signals are
        // per-`Update` sim messages), ordered `.after(SimSystems::Simulate)` so an outcome
        // written by the sim's `check_outcome` THIS update is read THIS update (no
        // one-frame lag — `bevy-traps.md` #3), and is presence-gated
        // (`not(resource_exists::<BattleRunningComplete>)`) so a census that re-declares
        // the outcome each tick inserts the marker at most once.
        .add_systems(
            Update,
            end_battle_on_outcome.after(SimSystems::Simulate).run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and_then(not(resource_exists::<BattleRunningComplete>)),
            ),
        )
        // GTW-334: `move_on` advances `BattleRunning → AnimateOut`, but DEFERRED past the
        // deciding shot's FX. The sim emits its `BattleWon` / `BattleLost` the SAME drain frame
        // the killing shot resolves, and `end_battle_on_outcome` latches `BattleRunningComplete`
        // that frame (the correct "outcome decided" latch — unchanged). The BUG was that the
        // transition fired the same frame, before the deciding tracer animates. `move_on` now
        // runs in `Update` (so it can read the presenter's FX-pipeline state each frame, which a
        // `FixedUpdate` placement could not order against), `.after(PresenterSystems::Draw)` so
        // this frame's FX spawn / advance / impact systems have already run, and holds the
        // transition until the deciding shot's projectile / impact pipeline has gone busy and
        // drained. It classifies a FLEE / non-shot end (no `ShotFired` at the latch) as PROMPT so
        // it never hangs on a pipeline that never goes busy (trap B); the spawn-race (trap A) is
        // closed by classifying off the latch-frame `ShotFired` (present even before the deferred
        // bolt materializes) and requiring busy-since-latch before idle counts as drained.
        .add_systems(
            Update,
            move_on.after(PresenterSystems::Draw).run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and_then(resource_exists::<BattleRunningComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            (print_on_exit, cleanup),
        );
}

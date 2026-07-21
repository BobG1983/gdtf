//! [`move_on`] — advance `BattleRunning → AnimateOut`, deferred until the battle's last
//! moments have actually been SHOWN (GTW-334, extended GTW-727 C39–C41).

use bevy::prelude::*;
use gdtf_battle_presenter::{PendingImpact, PlaybackGate, Played, ShotProjectile};
use gdtf_battle_sim::shot_fired::ShotFired;

use super::phase::{EndPhase, EndTransition, SeenBusy};
use crate::states::BattleScapeState;

/// `Update` (gated `in_state(BattleRunning) && resource_exists::<BattleRunningComplete>`,
/// ordered `.after(PresenterSystems::Draw)`): advance `BattleRunning → AnimateOut` once the
/// battle's ending has been shown.
///
/// ## What "shown" means, and why it changed
///
/// GTW-334 made this wait for the DECIDING shot's FX pipeline to drain, so the battle did
/// not end mid-tracer. That was sufficient while the sim's drain frame and the screen were
/// the same instant. Since GTW-727 they are not: the presenter replays an act log at its
/// own pace, so at the moment the outcome is decided there may be several unplayed entries
/// behind the deciding shot.
///
/// Two defects follow if this system only waits on the deciding shot's FX:
///
/// * **Silent truncation.** A lethal reaction volley resolves shots 1–3 in one tick. Shot 1
///   kills; the census decides; this gate releases at shot 1's impact; teardown drops the
///   log — and the player never sees shots 2 and 3. The ticket's headline behaviour, absent
///   in live play, with a green suite.
/// * **A hard hang.** If the deciding shot falls in a span the cursor skipped, its bolt
///   never spawns, the pipeline is never seen busy, and the FX branch waits forever — with
///   input gated shut and no in-battle quit key.
///
/// So the gate now has two conditions and a backstop:
///
/// 1. The playback cursor must have DRAINED the log ([`PlaybackGate::is_open`], C39). With
///    no presenter this is trivially true, so a headless run is unaffected.
/// 2. The deciding shot's FX pipeline must have gone busy and drained, as before.
/// 3. A bounded [`EndTransition`] backstop overrides both (C40): no phase may hold
///    indefinitely.
///
/// The classifier drains [`Played<ShotFired>`](Played) rather than the raw sim message
/// (C41), so a shot is classified as "deciding" at the moment the presenter first SHOWS it —
/// the same instant its bolt spawns. Reading the raw message would classify against a shot
/// that has not been drawn yet and start waiting for a pipeline that has not been asked to
/// do anything. The reader is drained every frame either way, so it never backs up.
///
/// Param-only (`bevy-traps.md` #7).
#[expect(
    clippy::too_many_arguments,
    reason = "the gate reads four independent facts before it may leave — the clock (its \
              backstop), the persisted phase, the playback cursor, and the FX pipeline (two \
              disjoint marker queries) — plus the shown-shot classifier and the two writes. \
              Each is a distinct Bevy SystemParam; bundling them would hide exactly the \
              inputs this decision is documented on"
)]
pub(in crate::states::running::game::battlescape::battle_running) fn move_on(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<NextState<BattleScapeState>>,
    transition: Option<ResMut<EndTransition>>,
    playback: PlaybackGate,
    projectiles: Query<(), With<ShotProjectile>>,
    pending: Query<(), With<PendingImpact>>,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    // The FX pipeline is BUSY while any bolt is in flight or any arrival seed is
    // outstanding. Once both are empty the deciding shot has flown and resolved.
    let pipeline_busy = !projectiles.is_empty() || !pending.is_empty();

    // This frame's shown shots MUST be read whether or not the phase already exists, so the
    // reader does not back up — but they only DECIDE the phase on the first latch frame.
    let deciding_shot_shown = shots.read().next().is_some();

    let mut transition = transition;
    let phase = if let Some(existing) = transition.as_deref_mut() {
        existing.tick(time.delta());
        existing.phase()
    } else {
        // First latch frame: classify, and fold in THIS frame's busy observation before
        // persisting — the resource is queued through `Commands`, so it is not readable
        // again until next frame and a busy read taken now would otherwise be lost.
        let initial = if deciding_shot_shown {
            EndPhase::AwaitingDecidingShot {
                seen_busy: SeenBusy::new(pipeline_busy),
            }
        } else {
            EndPhase::NoDecidingShot
        };
        commands.insert_resource(EndTransition::new(initial));
        initial
    };

    // C40: the backstop overrides everything. A wait that cannot complete ends the battle
    // rather than stranding the player in a state with no input and no way out.
    if transition
        .as_deref()
        .is_some_and(EndTransition::backstop_elapsed)
    {
        state.set(BattleScapeState::AnimateOut);
        return;
    }

    // C39: whatever the FX pipeline is doing, the battle does not end while the presenter
    // still has unplayed acts. This is what stops a lethal volley's later shots being
    // dropped by teardown before they are ever drawn.
    if !playback.is_open() {
        if pipeline_busy && let Some(existing) = transition.as_deref_mut() {
            existing.mark_seen_busy();
        }
        return;
    }

    match phase {
        // No tracer to wait for — leave at once (a flee / non-shot end must never hang on a
        // pipeline that never goes busy).
        EndPhase::NoDecidingShot => {
            state.set(BattleScapeState::AnimateOut);
        }
        EndPhase::AwaitingDecidingShot { seen_busy } => {
            if pipeline_busy {
                // The deciding bolt / impact is in flight — keep waiting, and record that
                // the pipeline has now been seen busy, so a later idle read is a true drain
                // rather than the pre-spawn gap.
                if let Some(existing) = transition.as_deref_mut() {
                    existing.mark_seen_busy();
                }
            } else if *seen_busy {
                // The pipeline went busy and has since drained: the deciding tracer landed
                // and its impact resolved, and the cursor has shown everything. Leave.
                state.set(BattleScapeState::AnimateOut);
            }
            // else: idle but never-yet-busy — the pre-spawn gap. Hold; do NOT treat this
            // idle as a drain.
        }
    }
}

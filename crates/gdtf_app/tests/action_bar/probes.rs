//! Message probes + selection/arming drivers shared across the action-bar suite.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    acts::{EndTurnRequested, SetAimingRequested, SetStanceRequested},
    ganger::{Aiming, Facing, TuMax},
    prelude::{Direction, Level, Stance, StanceKind},
    weapon::{FireMode, FireModeSpec, WieldedBy},
};
use gdtf_test_utils::{MessageProbe, MessageProbePlugin, drain_message_probe, probed};
use gdtf_ui::{DisabledButton, SegmentSubText};

use super::harness::*;

// ---------------------------------------------------------------------------------
// Message probes — collect the *Requested emitted this run into resources read in the
// test body (each probe runs after the drain, so it sees the same update's emission).
// ---------------------------------------------------------------------------------

/// Adds the two posture-message probes (GTW-576 `MessageProbePlugin<M>`) — the
/// `Last`-schedule drain observes the same update's emitted messages with its own
/// `MessageReader` cursor (independent of the sim's `dispatch_*`).
pub(crate) fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<SetStanceRequested>::default(),
        MessageProbePlugin::<SetAimingRequested>::default(),
    ));
}

/// The collected `SetStanceRequested` messages.
pub(crate) fn stances(app: &App) -> Vec<SetStanceRequested> {
    probed::<SetStanceRequested>(app)
}

/// The collected `SetAimingRequested` messages.
pub(crate) fn aims(app: &App) -> Vec<SetAimingRequested> {
    probed::<SetAimingRequested>(app)
}

/// Adds the `EndTurnRequested` probe — the generic GTW-576 `MessageProbe<M>` resource
/// with its drain registered at the ORIGINAL observation point (Update, AFTER the intent
/// drain): this harness runs the full battle runtime, whose enemy brain auto-passes an
/// empty enemy turn with a SECOND same-frame `EndTurnRequested`, so a `Last`-schedule
/// drain would count the brain's message too. The button test pins the DRAIN's emission.
pub(crate) fn add_end_turn_probe(app: &mut App) {
    app.init_resource::<MessageProbe<EndTurnRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<EndTurnRequested>.after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected `EndTurnRequested` messages.
pub(crate) fn end_turns(app: &App) -> Vec<EndTurnRequested> {
    probed::<EndTurnRequested>(app)
}

/// Spawns a ganger carrying exactly the GANGER components the act / panel systems read
/// (`Stance`/`Facing`/`Aiming`) plus a related WEAPON entity carrying the `FireMode`
/// selector (`ganger → Wields → weapon`, GTW-323 slice 3 — the mode panel reads the
/// offered modes off the weapon entity now, not the ganger), and SELECTS the ganger via
/// the `SelectedShooter` resource. Returns the ganger entity. (The act surface only reads
/// `*SelectedShooter`, so setting the resource directly is the faithful, minimal selection
/// for these button tests; the cursor-click selection path is covered in
/// `gdtf_battle_input`'s `acts.rs`.) The `WieldedBy` insert hook populates the ganger's
/// `Wields` synchronously in a bare `World` spawn, so the next update resolves the weapon.
pub(crate) fn arm_and_select(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((Stance::new(stance), Facing::new(facing), Aiming::new(false)))
        .id();
    app.world_mut().spawn((WieldedBy::new(ganger), selector));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// Like [`arm_and_select`], but ALSO gives the selected GANGER a [`TuMax`] (the round-start
/// ceiling the per-shot TU charge is a percentage of) — the input GTW-303's cost-line system
/// reads off the selected shooter. `arm_and_select` omits `TuMax` (the posture/mode tests do
/// not need it), so the cost-line tests use this variant. The `FireMode` selector rides on
/// the related weapon entity (`Wields`, GTW-323 slice 3). Returns the ganger entity.
pub(crate) fn arm_and_select_with_tu(
    app: &mut App,
    selector: FireMode,
    tu_max: TuMax,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Stance::new(stance),
            Facing::new(facing),
            Aiming::new(false),
            tu_max,
        ))
        .id();
    app.world_mut().spawn((WieldedBy::new(ganger), selector));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// The TU-cost SUB-LINE text of the firemode SEGMENT carrying marker `M`, if exactly one
/// such segment exists and it has a [`SegmentSubText`] child (GTW-303): the string the
/// `set_segment_sub_line` slice-1 path wrote below that segment's mode-name label.
///
/// Returns `None` when the segment has no sub-line node (the cost line was cleared, e.g. an
/// unarmed selection or a non-offered mode), which is how the "no cost line" cases assert.
pub(crate) fn segment_sub_line<M: Component>(app: &mut App) -> Option<String> {
    let segment = single_with::<M>(app)?;
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(segment)
        .map(|kids| kids.iter().collect())
        .unwrap_or_default();
    children.into_iter().find_map(|child| {
        if app.world().get::<SegmentSubText>(child).is_some() {
            app.world().get::<Text>(child).map(|t| t.0.clone())
        } else {
            None
        }
    })
}

/// Sets the selected ganger's [`Aiming`] flag (marking it `Changed`, so the cost-line system
/// re-derives), then settles a couple of updates so the live cost lines reflect the new aim
/// state. The selection is the `SelectedShooter` resource.
pub(crate) fn set_selected_aiming(app: &mut App, aiming: bool) {
    let Some(shooter) = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
    else {
        return;
    };
    if let Some(mut aim) = app.world_mut().get_mut::<Aiming>(shooter) {
        *aim = Aiming::new(aiming);
    }
    app.update();
    app.update();
}

/// The current `ActiveLevel` storey as a plain `u8`, if present.
pub(crate) fn active_level(app: &App) -> Option<u8> {
    app.world().get_resource::<ActiveLevel>().map(|l| *(**l))
}

/// Whether the single button carrying marker `M` currently carries the `gdtf_ui`
/// [`DisabledButton`] marker (GTW-293 — the greyed/disabled state). `false` if no such
/// single button exists. The bounds-disable test reads this off each level button after
/// driving `ActiveLevel` to a bound.
pub(crate) fn is_disabled<M: Component>(app: &mut App) -> bool {
    single_with::<M>(app).is_some_and(|button| app.world().get::<DisabledButton>(button).is_some())
}

/// Sets `ActiveLevel` to `storey` and settles one update so the bounds-disable system
/// (`sync_level_button_bounds`) reacts and writes the `DisabledButton` markers (GTW-293).
pub(crate) fn set_active_level(app: &mut App, storey: u8) {
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(storey)));
    app.update();
}

/// The current `SelectedFireMode` spec, if present.
pub(crate) fn selected_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}

use bevy::{ecs::entity::Entity, prelude::*};
use cobalt_test_utils::{MessageProbe, MessageProbePlugin, drain_message_probe, probed};
use gdtf_battle_input::{ChosenFireMode, SelectedShooter, chosen_spec};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    acts::{EndTurnRequested, SetAimingRequested, SetStanceRequested},
    ganger::{Aiming, Facing, TuMax},
    prelude::{Direction, Level, Stance, StanceKind},
    weapon::{FireMode, FireModeSpec, MeleeWeapon, MountedWeapon, WieldedBy, Wields},
};
use gdtf_ui::{DisabledButton, SegmentSubText};

use super::harness::*;

pub(crate) fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<SetStanceRequested>::default(),
        MessageProbePlugin::<SetAimingRequested>::default(),
    ));
}

pub(crate) fn stances(app: &App) -> Vec<SetStanceRequested> {
    probed::<SetStanceRequested>(app)
}

pub(crate) fn aims(app: &App) -> Vec<SetAimingRequested> {
    probed::<SetAimingRequested>(app)
}

pub(crate) fn add_end_turn_probe(app: &mut App) {
    app.init_resource::<MessageProbe<EndTurnRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<EndTurnRequested>.after(gdtf_battle_input::dispatch_act_intents),
    );
}

pub(crate) fn end_turns(app: &App) -> Vec<EndTurnRequested> {
    probed::<EndTurnRequested>(app)
}

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

pub(crate) fn active_level(app: &App) -> Option<u8> {
    app.world().get_resource::<ActiveLevel>().map(|l| *(**l))
}

pub(crate) fn is_disabled<M: Component>(app: &mut App) -> bool {
    single_with::<M>(app).is_some_and(|button| app.world().get::<DisabledButton>(button).is_some())
}

pub(crate) fn set_active_level(app: &mut App, storey: u8) {
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(storey)));
    app.update();
}

/// The spec the gun the selected shooter fires is set to.
pub(crate) fn selected_mode(app: &App) -> Option<FireModeSpec> {
    let shooter = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)?;
    let wields = app.world().get::<Wields>(shooter)?;
    let weapon = wields.firing_weapon(
        |weapon| {
            app.world()
                .get_entity(weapon)
                .is_ok_and(|row| row.contains::<MountedWeapon>())
        },
        |weapon| {
            app.world()
                .get_entity(weapon)
                .is_ok_and(|row| row.contains::<MeleeWeapon>())
        },
    )?;
    let modes = app.world().get::<FireMode>(weapon)?;
    chosen_spec(modes, app.world().get::<ChosenFireMode>(weapon))
}

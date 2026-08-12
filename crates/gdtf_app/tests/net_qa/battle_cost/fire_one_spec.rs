//! `battle.cost {Fire}` and `battle.sightline` resolve one spec, not one kind.
//!
//! Both read the gun's own entry, so a `SelectedFireMode` the gun no longer matches cannot make
//! them answer differently, and a gun carrying no `FireMode` at all is refused by both.

use bevy::{
    app::App,
    ecs::{entity::Entity, relationship::RelationshipTarget},
};
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    misc::ModeKindNet,
};
use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::{
    ganger::{Direction, Facing, Tu},
    magazine::Magazine,
    weapon::{
        FireMode, FireModeSpec, Handedness, MeleeWeapon, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Wields,
    },
};
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};

use super::support::{cost_body, cost_call, fire_cost, settle};
use crate::{
    battle_fixture::{arm_selected_with_modes, selected_shooter},
    battle_reads::{cell_argument, cell_of, one_step_from, token_of},
    battle_sightline::{engage_answer, sightline_body},
    command_exchange::{BATTLE_SIGHTLINE, exchange_in_battle, run},
    magazine_support::fill_the_magazine,
    socket_support::{TestError, TestResult},
};

/// Why neither case can run on a fixture that never armed a shooter with a clear cell beside it.
const NO_SHOT: &str = "a running battle must select a player ganger holding a gun, with a clear \
                       cell beside it to be asked about";

/// What the gun charges for a shot before a case rewrites its mode list in place.
const DEAR: ModeTuPercent = ModeTuPercent::new(0.9);

/// What it charges afterwards, which is what both commands have to price from.
const CHEAP: ModeTuPercent = ModeTuPercent::new(0.1);

/// The single mode this gun offers, at the price the case gives it.
fn one_mode_gun(tu_percent: ModeTuPercent) -> FireMode {
    FireMode::new(vec![one_mode(tu_percent)])
}

/// That mode on its own, for pricing a shot the same way the gun would.
const fn one_mode(tu_percent: ModeTuPercent) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        tu_percent,
        ModeShots::new(1),
    )
}

/// The shooter both commands read and the cell they are asked about.
struct Asked {
    shooter: Entity,
    at:      CellLevelNet,
}

/// The two requests one case sends, in the order the answers come back.
fn both_calls(asked: &Asked) -> Option<Vec<QaRequest>> {
    let act = CostActNet::Fire {
        target: asked.at,
        mode:   ModeKindNet::from_sim(ModeKind::Single),
    };
    Some(vec![
        cost_call(token_of(asked.shooter), &act)?,
        run(
            BATTLE_SIGHTLINE,
            &cell_argument(asked.at),
            RunOptions::default(),
        ),
    ])
}

/// The weapon the selected shooter fires.
fn the_gun_of(app: &App) -> Option<Entity> {
    let shooter = selected_shooter(app);
    let held: Vec<Entity> = app
        .world()
        .get::<Wields>(shooter)
        .map(|wields| wields.iter().collect())
        .unwrap_or_default();
    held.into_iter()
        .find(|weapon| app.world().get::<MeleeWeapon>(*weapon).is_none())
}

/// Turn the shooter toward `at`, load it, and leave it holding exactly `budget`.
fn ready_to_shoot(app: &mut App, shooter: Entity, at: CellLevelNet, budget: Tu) -> Option<()> {
    let standing = cell_of(app, shooter)?;
    let facing = Direction::from_cells(standing.to_sim().cell(), at.to_sim().cell())?;
    let _rounds = fill_the_magazine(app, shooter)?;
    app.world_mut()
        .get_entity_mut(shooter)
        .ok()?
        .insert((Facing::new(facing), budget));
    Some(())
}

/// The spec the resource holds, and the one the gun offers for that same kind.
fn held_and_offered(app: &App) -> Option<(FireModeSpec, FireModeSpec)> {
    let held = **app.world().get_resource::<SelectedFireMode>()?;
    let offered = app
        .world()
        .get_entity(the_gun_of(app)?)
        .ok()?
        .get::<FireMode>()?
        .iter()
        .find(|spec| spec.kind == held.kind)
        .copied()?;
    Some((held, offered))
}

/// Arm the shooter, settle so the resource copies that gun, then rewrite the gun in place.
///
/// Mutating `FireMode` where it stands fires none of the sync's triggers, so the resource keeps
/// the dear spec while the gun goes cheap — two specs of one kind, at different prices.
fn pose_a_stale_resource(app: &mut App) -> Option<Asked> {
    let gun = arm_selected_with_modes(app, one_mode_gun(DEAR));
    settle(app);
    app.world_mut()
        .get_entity_mut(gun)
        .ok()?
        .insert(one_mode_gun(CHEAP));
    let shooter = selected_shooter(app);
    let at = one_step_from(app, cell_of(app, shooter)?)?;
    let cheap_shot = fire_cost(app, shooter, &one_mode(CHEAP))?;
    ready_to_shoot(app, shooter, at, cheap_shot)?;
    Some(Asked { shooter, at })
}

/// Arm the shooter, settle, then take the mode list off the gun it fires and leave the rest.
fn pose_a_gun_with_no_modes(app: &mut App) -> Option<Asked> {
    let gun = arm_selected_with_modes(app, one_mode_gun(CHEAP));
    settle(app);
    app.world_mut()
        .get_entity_mut(gun)
        .ok()?
        .remove::<FireMode>();
    let shooter = selected_shooter(app);
    let at = one_step_from(app, cell_of(app, shooter)?)?;
    ready_to_shoot(app, shooter, at, Tu::new(u8::MAX))?;
    let row = app.world().get_entity(gun).ok()?;
    assert!(
        row.contains::<Magazine>() && row.contains::<Handedness>(),
        "the gun must keep everything but its mode list, or the case proves a missing magazine \
         rather than a missing FireMode",
    );
    Some(Asked { shooter, at })
}

/// Pose the world, ask both commands about one shot, and hand back what they answered.
fn ask_both(
    pose: impl FnOnce(&mut App) -> Option<Asked>,
) -> Result<(App, Vec<QaResponse>), TestError> {
    let mut posed = false;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(asked) = pose(app) else {
            return Vec::new();
        };
        let calls = both_calls(&asked).unwrap_or_default();
        posed = !calls.is_empty();
        calls
    })?;
    if !posed {
        return Err(TestError::from(NO_SHOT));
    }
    Ok((app, replies))
}

/// The two answers one case gets back, in the order they were asked for.
fn two_answers(replies: Vec<QaResponse>) -> Result<(QaResponse, QaResponse), TestError> {
    let mut answers = replies.into_iter();
    match (answers.next(), answers.next()) {
        (Some(quote), Some(sight)) => Ok((quote, sight)),
        other => Err(format!("both commands must come back with a reply, got {other:?}").into()),
    }
}

#[test]
fn a_selected_mode_the_gun_no_longer_prices_still_leaves_the_two_commands_agreeing() -> TestResult {
    let (app, replies) = ask_both(pose_a_stale_resource)?;

    let Some((held, offered)) = held_and_offered(&app) else {
        return Err("the shooter must still hold the gun the case armed it with".into());
    };
    assert_eq!(
        held.kind, offered.kind,
        "the case is about one kind resolving to two specs, so both sides must still name the \
         same kind: {held:?} against {offered:?}",
    );
    assert_ne!(
        held.tu_percent, offered.tu_percent,
        "the resource must still hold the spec the gun dropped, or there is nothing here to tell \
         apart: {held:?} against {offered:?}",
    );

    let (quote, sight) = two_answers(replies)?;
    let quoted = cost_body(quote)?;
    let seen = sightline_body(Some(sight))?;
    let engaged = engage_answer(&seen)?;
    assert_eq!(
        *quoted.legal, *engaged,
        "both commands price the gun's own entry for the mode, so a shooter left holding exactly \
         what that entry costs is allowed by both — {quoted:?} against {seen:?}",
    );
    assert!(
        *engaged,
        "the gun's own entry is affordable, so the shot is on; a refusal here is the spec the \
         resource kept being priced instead — {quoted:?} against {seen:?}",
    );
    Ok(())
}

#[test]
fn a_wielded_gun_with_no_fire_mode_is_refused_by_both_commands() -> TestResult {
    let (app, replies) = ask_both(pose_a_gun_with_no_modes)?;
    assert!(
        the_gun_of(&app)
            .and_then(|gun| app.world().get_entity(gun).ok())
            .is_some_and(|row| !row.contains::<FireMode>()),
        "the gun must still be carrying no mode list when the answers came back",
    );

    let (quote, sight) = two_answers(replies)?;
    let quoted = cost_body(quote)?;
    let seen = sightline_body(Some(sight))?;
    let engaged = engage_answer(&seen)?;
    assert_eq!(
        quoted.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "a gun offering no mode prices no shot: {quoted:?}",
    );
    assert!(
        !*engaged,
        "the same gun cannot be engaged with either, or the two commands disagree about a shot \
         one of them will not price — {quoted:?} against {seen:?}",
    );
    assert_eq!(
        *quoted.legal, *engaged,
        "both commands read the same gun row, so their verdicts match — {quoted:?} against \
         {seen:?}",
    );
    Ok(())
}

//! One shot, two commands: `battle.cost {Fire}` and `battle.sightline` must answer alike across
//! every combination of firing arc, time units and rounds in the magazine.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::{command::RunOptions, message::McpResponse};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::{Direction, Facing, Tu},
    weapon::ModeKind,
};
use gdtf_game::qa_wire::{cell::CellLevelNet, cost::CostActNet, misc::ModeKindNet};

use super::support::{cost_body, cost_call, fire_cost, settle};
use crate::mcp::{
    battle_reads::{cell_argument, cell_of, one_step_from, player_gangers, token_of},
    battle_setup::MagazineLoad,
    battle_sightline::{engage_answer, sightline_body},
    command_exchange::{BATTLE_SIGHTLINE, exchange_in_battle, run},
    magazine_support::{empty_the_magazine, fill_the_magazine},
    socket_support::{TestError, TestResult},
};

/// More time units than any turn-and-shot ever costs.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

/// Whether the shooter is turned toward the cell it is asked about or away from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arc {
    /// Facing straight at the cell, so no turn is priced.
    Toward,
    /// Facing straight away from it, so a turn is priced first.
    Away,
}

/// What the shooter has left to spend when it is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Budget {
    /// More than any shot costs.
    Ample,
    /// Exactly the shot, with nothing over for turning.
    ShotOnly,
    /// Nothing at all.
    Nothing,
}

/// One cell of the table both commands are asked in.
#[derive(Debug, Clone, Copy)]
struct Case {
    arc:    Arc,
    budget: Budget,
    load:   MagazineLoad,
}

const fn case(arc: Arc, budget: Budget, load: MagazineLoad) -> Case {
    Case { arc, budget, load }
}

/// Every combination of arc, pool and magazine the two commands can be asked about.
const TABLE: [Case; 12] = [
    case(Arc::Toward, Budget::Ample, MagazineLoad::Loaded),
    case(Arc::Toward, Budget::Ample, MagazineLoad::Empty),
    case(Arc::Toward, Budget::ShotOnly, MagazineLoad::Loaded),
    case(Arc::Toward, Budget::ShotOnly, MagazineLoad::Empty),
    case(Arc::Toward, Budget::Nothing, MagazineLoad::Loaded),
    case(Arc::Toward, Budget::Nothing, MagazineLoad::Empty),
    case(Arc::Away, Budget::Ample, MagazineLoad::Loaded),
    case(Arc::Away, Budget::Ample, MagazineLoad::Empty),
    case(Arc::Away, Budget::ShotOnly, MagazineLoad::Loaded),
    case(Arc::Away, Budget::ShotOnly, MagazineLoad::Empty),
    case(Arc::Away, Budget::Nothing, MagazineLoad::Loaded),
    case(Arc::Away, Budget::Nothing, MagazineLoad::Empty),
];

/// The shooter the game selected, the cell both commands are asked about, and the mode in force.
struct Asked {
    shooter: Entity,
    at:      CellLevelNet,
    mode:    ModeKind,
}

/// Pose the shooter the game already selected for one cell of the table.
fn pose(app: &mut App, case: Case) -> Option<Asked> {
    settle(app);
    let shooter = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)?;
    if !player_gangers(app).contains(&shooter) {
        return None;
    }
    let standing = cell_of(app, shooter)?;
    let at = one_step_from(app, standing)?;
    let mode = crate::mcp::fire_mode_support::selected_mode(app)?;
    let (from, to) = (standing.to_sim().cell(), at.to_sim().cell());
    let facing = match case.arc {
        Arc::Toward => Direction::from_cells(from, to)?,
        Arc::Away => Direction::from_cells(to, from)?,
    };
    let budget = match case.budget {
        Budget::Ample => AMPLE_TU,
        Budget::ShotOnly => fire_cost(app, shooter, &mode)?,
        Budget::Nothing => Tu::new(0),
    };
    app.world_mut()
        .get_entity_mut(shooter)
        .ok()?
        .insert((Facing::new(facing), budget));
    match case.load {
        MagazineLoad::Loaded => fill_the_magazine(app, shooter),
        MagazineLoad::Empty => empty_the_magazine(app, shooter),
    }?;
    Some(Asked {
        shooter,
        at,
        mode: mode.kind,
    })
}

/// The two requests one cell of the table sends, in the order the answers come back.
fn both_calls(asked: &Asked) -> Option<Vec<cobalt_mcp_protocol::message::McpRequest>> {
    let act = CostActNet::Fire {
        target: asked.at,
        mode:   ModeKindNet::from_sim(asked.mode),
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

/// Ask both commands about the same shot, in one exchange, and hand back what they answered.
fn ask_both(case: Case) -> Result<(App, Asked, Vec<McpResponse>), TestError> {
    let mut planned: Option<Asked> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(asked) = pose(app, case) else {
            return Vec::new();
        };
        let calls = both_calls(&asked).unwrap_or_default();
        planned = Some(asked);
        calls
    })?;
    let Some(asked) = planned else {
        return Err(TestError::from(
            "a running battle must select a player ganger holding a gun, with a clear cell beside \
             it to be asked about",
        ));
    };
    Ok((app, asked, replies))
}

/// Both commands must have been reading one shooter and one fire mode when they answered.
fn assert_one_shot_was_asked_about(app: &App, asked: &Asked, case: Case) {
    let world = app.world();
    assert_eq!(
        world
            .get_resource::<SelectedShooter>()
            .and_then(|selected| **selected),
        Some(asked.shooter),
        "{case:?}: the selection must still name the ganger the quote was asked about, or the two \
         commands were reading different shooters",
    );
    assert_eq!(
        crate::mcp::fire_mode_support::selected_mode(app).map(|mode| mode.kind),
        Some(asked.mode),
        "{case:?}: the mode the gun is on must still be the one the quote named, or the two \
         commands were pricing different shots",
    );
}

#[test]
fn the_cost_quote_and_the_sightline_agree_about_one_shot() -> TestResult {
    for case in TABLE {
        let (app, asked, replies) = ask_both(case)?;
        assert_one_shot_was_asked_about(&app, &asked, case);
        let mut answers = replies.into_iter();
        let (Some(quote), Some(sight)) = (answers.next(), answers.next()) else {
            return Err(format!("{case:?}: both commands must come back with a reply").into());
        };
        let quoted = cost_body(quote)?;
        let seen = sightline_body(Some(sight))?;
        let engaged = engage_answer(&seen)?;
        assert_eq!(
            *quoted.legal, *engaged,
            "{case:?}: battle.cost and battle.sightline answer about the same shot, so their \
             verdicts must match — {quoted:?} against {seen:?}",
        );
    }
    Ok(())
}

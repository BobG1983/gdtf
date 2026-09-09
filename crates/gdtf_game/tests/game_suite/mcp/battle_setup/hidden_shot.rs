//! A battle whose log holds acts by an enemy the squad cannot see.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    acts::{FireDeclaration, RoundCount},
    cover::HeightBand,
    metric::cell_center,
    prelude::{Cell, CellLevel, Level},
    resolve_coarse::{ShotKind, ShotOutcome},
    sample_cone::ShotDir,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    weapon::{DamageType, ModeKind},
};

use super::{
    catch_up::let_the_screen_catch_up,
    expected::HiddenShooter,
    fixtures::stand_at,
    map::{a_row_across_the_lit_area, live_fog, settle},
};
use crate::mcp::{
    battle_reads::an_enemy_ganger,
    socket_support::{TestError, battle_app_listening},
};

/// Rounds the hidden shooter declares, so the declaration claims exactly one resolved round.
const ONE_ROUND: u32 = 1;

/// A live battle whose log holds two acts by an enemy standing where the squad cannot see it:
/// one on the enemy's own unlit cell, one a shot across a watched cell into another unlit cell.
pub(crate) fn battle_with_a_hidden_enemy_shooting_across_the_lit_area()
-> Result<(App, McpPort, HiddenShooter), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some((muzzle, lit, impact)) = a_row_across_the_lit_area(&app) else {
        return Err(
            "the generated map must offer one row holding an unlit stand cell, a lit cell \
             past it, and an unlit cell past that"
                .into(),
        );
    };
    stand_at(&mut app, entity, muzzle, muzzle)?;
    let_the_screen_catch_up(&mut app);
    for (at, want_lit) in [(muzzle, false), (lit, true), (impact, false)] {
        let is_lit = live_fog(&app).is_some_and(|fog| *fog.is_cell_visible(&at));
        if is_lit != want_lit {
            return Err(format!(
                "the sim's own fog must agree with the screen before any act is logged: \
                 {at:?} is lit={is_lit}, wanted lit={want_lit}"
            )
            .into());
        }
    }

    app.world_mut()
        .write_message(SuppressionApplied::new(entity, muzzle));
    app.world_mut().write_message(FireDeclaration::new(
        entity,
        None,
        ModeKind::Single,
        RoundCount::new(ONE_ROUND),
    ));
    let (impact_cell, impact_level) = impact.split();
    app.world_mut()
        .write_message(a_round(entity, muzzle, impact_cell, impact_level));
    app.update();

    Ok((app, port, HiddenShooter { entity }))
}

/// A resolved round leaving `muzzle` and stopping at the named cell, hitting nothing.
fn a_round(shooter: Entity, muzzle: CellLevel, cell: Cell, level: Level) -> ShotFired {
    let (from, from_level) = muzzle.split();
    let start = cell_center(from, from_level);
    let end = cell_center(cell, level);
    let outcome = ShotOutcome {
        kind: ShotKind::Miss,
        cell,
        level,
        body_part: None,
        band: HeightBand::Mid,
        muzzle: start,
        trajectory: ShotDir::from_direction(*end - *start),
    };
    ShotFired::from_outcome(shooter, DamageType::Kinetic, &outcome)
}

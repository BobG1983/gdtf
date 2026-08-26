//! Battle fixtures and what they report back.

mod behind;
mod catch_up;
mod expected;
mod fixtures;
mod hidden_shot;
mod map;
mod modes;
mod terrain;

pub(crate) use behind::{
    battle_with_a_frozen_fog, battle_with_a_ganger_the_screen_has_not_moved,
    battle_with_a_player_ganger_the_screen_has_not_moved,
    battle_with_an_occupant_the_screen_has_not_seen, battle_with_remembered_cover,
    forget_on_screen_only, hold_the_screen_still,
};
pub(crate) use catch_up::{let_the_screen_catch_up, the_screen_has_caught_up};
pub(crate) use expected::{
    ExpectedEnemy, FLOODED_LOG_LINES, IdlePair, LOG_LINES_WRITTEN, LoggedActor, MagazineLoad,
    SpawnedDoor, Standing,
};
pub(crate) use fixtures::{
    battle_reporting_a_player_card, battle_reporting_an_enemy, battle_with_a_door,
    battle_with_a_flooded_log, battle_with_a_selected_fire_mode,
    battle_with_a_shooter_facing_north, battle_with_an_enemy,
    battle_with_an_enemy_beside_an_idle_ganger, battle_with_another_gang_acting,
    battle_with_lit_cover, battle_with_log_lines,
};
pub(crate) use hidden_shot::battle_with_a_hidden_enemy_shooting_across_the_lit_area;
pub(crate) use map::a_lit_cover_cell;
pub(crate) use modes::battle_with_a_two_mode_gun;
pub(crate) use terrain::{
    battle_with_a_lit_emplacement, battle_with_a_lit_unledgered_wall,
    manned_emplacement_on_authored_terrain, shown_fog_lights,
};

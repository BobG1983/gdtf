//! The terrain-draw app builds, the setup driver, the world authoring, the playback
//! helpers, and the shared tile probes.

mod build;
mod playback;
mod probes;
mod setup;
mod world;

pub(crate) use build::{
    headless_renderer_app, headless_renderer_app_at, headless_renderer_app_without_raw_destroyed,
    settle_resources, write_sprite_def,
};
pub(crate) use playback::{
    detained_smash_log, holding, moved_to_log, play_past, raw_destroyed_present, shown,
};
pub(crate) use probes::{
    def_rect, sprite_defs, sprite_entity_at, sprite_rect_at, stamped_graphic_at,
    terrain_sprite_count_on_level,
};
pub(crate) use setup::{drive_setup, spawn_test_ganger};
pub(crate) use world::{
    CENTER_WALL_DEF, despawn_terrain_entity, draw_one_wall, insert_occupancy, low_cover_entry,
    spawn_piece_entity, spawn_terrain_entity,
};

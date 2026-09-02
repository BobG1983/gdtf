//! HARNESS NOTE (the `melee_cover_smash` idiom): the sim is the LOW crate (a dev-dep on
//! `gdtf_test_utils` would be a cycle), so `harness` builds the `App` and adds `BattleSimPlugin`.
mod blocking;
mod cost;
mod death;
mod duplicate_requests;
mod eject_landing;
mod eject_on_destroy;
mod enter_exit;
mod entry_sides;
mod harness;
mod mounted_cover;
mod mounted_fire;
mod rejections;
mod same_frame;
mod seeded_sides;
mod sight;
mod trapped;
mod walk_off;
mod zero_step;

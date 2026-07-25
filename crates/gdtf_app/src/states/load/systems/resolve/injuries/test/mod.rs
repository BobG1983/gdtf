//! GTW-437 loader tests — the injury registry + tables build, the per-source
//! [`DamageContext`](gdtf_battle_sim::injuries::DamageContext) bucket keying (GTW-452),
//! the WARN-not-fail handling, the canonical-sort determinism, and the headless
//! hot-reload rebuild.
//!
//! These drive the REAL loader code path: the headless hot-reload test runs the actual
//! [`redrive_injuries_on_asset_event`] system inside a `MinimalPlugins` + `AssetPlugin`
//! app (the weapons-test harness, generalised to two asset types), and the build /
//! determinism / WARN tests exercise the actual private [`build_injury_data`] /
//! `build_tables` against in-memory assets. Per the loader-tests rule they assert
//! STRUCTURE (parse-OK / key-resolves / determinism / WARN-not-fail), never specific
//! shipped tunable magnitudes.

mod support;

mod build;
mod context;
mod hot_reload;
mod warns;

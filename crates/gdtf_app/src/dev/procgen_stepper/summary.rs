//! The stepper panel's pure stage-summary formatter (GTW-655).
//!
//! Split out of `super::ui` (rather than living inline in the egui draw system) so it
//! compiles — and is UNIT TESTABLE — under `test-support` too: `super::ui` itself needs a
//! primary egui context the headless harness has none of (bevy-traps #8), but this formatter
//! is a pure `&StagedProcgen -> String` mapping with no egui/Bevy-system dependency at all,
//! so it carries no reason to live behind that same exclusion. A plain code span above, not a
//! doc link — `ui` is `#[cfg(not(feature = "test-support"))]`-gated, so a bracketed intra-doc
//! link to it would break exactly where this file's own doc is checked
//! (`cargo doc --features test-support`).

use gdtf_battle_sim::procgen::{ProcgenStage, StagedProcgen};

crate::support_item! {
    /// A short, human-readable summary of what the driver's current position implies — the
    /// stage that just ran (or is about to run) and what it produced, for the two stages
    /// (assemble / fill) the real presenter cannot draw yet (see `super::ui`'s module doc — a
    /// plain code span, not a doc link: `ui` is `#[cfg(not(feature = "test-support"))]`-gated,
    /// so a bracketed link here would break under `cargo doc --features test-support`, which
    /// DOES compile this item's doc since it is `pub` in that configuration).
    /// `pub` under `test-support` (this file's own tests, plus any future integration test,
    /// drive it directly), `pub(crate)` otherwise (`super::ui`'s draw system calls it).
    #[must_use]
    fn stage_summary(driver: &StagedProcgen) -> String {
        match driver.stage() {
            ProcgenStage::Assemble => {
                "Not started: Next runs the assemble stage (player anchor + opposite enemy \
                 placement)."
                    .to_owned()
            }
            ProcgenStage::Fill => driver.placement().map_or_else(
                || "Assemble stage pending.".to_owned(),
                |placement| {
                    format!(
                        "Assembled: player \"{}\" at {:?}, enemy \"{}\" at {:?} (seam-separated). \
                         Next runs the fill stage.",
                        **placement.player().prefab().name(),
                        placement.player().anchor(),
                        **placement.enemy().prefab().name(),
                        placement.enemy().anchor(),
                    )
                },
            ),
            ProcgenStage::Emit => driver.filled().map_or_else(
                || "Fill stage pending.".to_owned(),
                |filled| {
                    format!(
                        "Filled: {} interior prefab(s) placed, {} dead-space region(s) floored. \
                         Next runs the emit stage (the map appears once it completes).",
                        filled.fill().len(),
                        filled.dead_space().len(),
                    )
                },
            ),
            ProcgenStage::Done => driver.emitted().map_or_else(
                || {
                    driver.failure().map_or_else(
                        || "Done.".to_owned(),
                        |err| format!("Failed closed: {err} — the authored terrain will be used."),
                    )
                },
                |emitted| {
                    format!(
                        "Emitted: {} wall(s), {} slab(s), {} floor tile(s). Finishing the battle \
                         setup.",
                        emitted.situation.walls.len(),
                        emitted.situation.slabs.len(),
                        emitted.situation.floors.len(),
                    )
                },
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use gdtf_battle_sim::{
        level::{
            GridHeight, GridLevels, GridSize, GridWidth, PrefabRegistry, ThemeUuid,
            UuidThemeRegistry,
        },
        procgen::{ProcgenTuning, StagedProcgen, StagedProcgenRegistries},
        rng::BattleSeed,
        terrain::def::TerrainDefRegistry,
    };

    use super::stage_summary;

    /// A minimal board size for every test in this module — no prefab needs to fit it (the
    /// failure-branch fixture registers none, and the not-started branch never reaches
    /// placement), so any in-bounds span will do.
    fn board() -> Option<GridSize> {
        GridSize::new(GridWidth::new(24), GridHeight::new(24), GridLevels::new(1)).ok()
    }

    /// A freshly-constructed driver (no `advance` call yet) summarizes the Assemble stage as
    /// "not started" — reachable with NO registries at all, since `stage()` alone decides
    /// this branch.
    #[test]
    fn assemble_not_started_names_the_next_stage() {
        let Some(grid_size) = board() else {
            return;
        };
        let driver = StagedProcgen::new(BattleSeed::new(1), ThemeUuid::nil(), grid_size);
        assert!(
            stage_summary(&driver).contains("Not started"),
            "a driver that has never advanced must summarize as not-yet-started",
        );
    }

    /// A driver whose assemble stage fails closed (an EMPTY `PrefabRegistry` — no player/enemy
    /// prefab can ever place) summarizes as "Failed closed" — mirrors the sim's own
    /// `advance_on_failure_is_terminal_and_repeats_the_same_error` fixture
    /// (`gdtf_battle_sim::lifecycle::procgen::test::staged`).
    #[test]
    fn done_with_failure_names_the_error() {
        let Some(grid_size) = board() else {
            return;
        };
        let prefabs = PrefabRegistry::default();
        let themes = UuidThemeRegistry::default();
        let terrain_defs = TerrainDefRegistry::default();
        let tuning = ProcgenTuning::default();
        let registries = StagedProcgenRegistries {
            prefabs:      &prefabs,
            themes:       &themes,
            terrain_defs: &terrain_defs,
            tuning:       &tuning,
        };
        let mut driver = StagedProcgen::new(BattleSeed::new(1), ThemeUuid::nil(), grid_size);
        let _advanced = driver.advance(registries);
        assert!(
            driver.is_done(),
            "an empty PrefabRegistry must fail the assemble stage closed immediately",
        );
        assert!(
            stage_summary(&driver).starts_with("Failed closed"),
            "a failed-closed drive must summarize the error, not the empty-success text",
        );
    }
}

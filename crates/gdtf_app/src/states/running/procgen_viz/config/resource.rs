//! [`VizConfig`] — the DEV-ONLY procgen-visualizer's SELECTED INPUTS (GTW-498).
//!
//! The configurable theme / grid-size / seed / gangs the input panel drives and the Generate
//! action ([`super::apply::generate_on_press`]) re-runs the procgen pipeline with (C1–C5). It
//! is the seam between the input widgets (which MUTATE its fields on a selection / commit) and
//! the rebuild (which READS it). Inserted `OnEnter(DebugProcgenVisualizer)` seeded from the
//! loaded situation + default seed (C6: the initial state matches the prior GTW-434 behaviour)
//! and removed `OnExit` (the state-scoped-resource convention, `bevy-traps.md` #1).
//!
//! Every field is a TYPED domain value (no-bare-types): [`ThemeUuid`] / [`BattleSeed`] /
//! [`GangName`] reuse the sim newtypes; the three grid axes are held as the sim's
//! [`GridWidth`] / [`GridHeight`] / [`GridLevels`] (each its own validated newtype) rather than
//! a [`GridSize`] directly, because the panel edits each axis independently and an
//! in-progress combo may be momentarily invalid — [`grid_size`](VizConfig::grid_size) folds the
//! three axes back through [`GridSize::new`] (the validated, never-panicking constructor) so an
//! invalid combo is SURFACED (C2) rather than silently clamped.
//!
//! The whole module is `#[cfg(debug_assertions)]`-gated by its parent (`procgen_viz`).

use bevy::prelude::*;
use gdtf_battle_sim::{
    GangName, GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, ThemeUuid,
    rng::BattleSeed,
};

use crate::states::running::procgen_viz::model::default_viz_seed;

crate::support_item! {
    /// The DEV-ONLY procgen-visualizer's selected inputs (GTW-498) — the theme, the three
    /// grid-size axes, the seed, and the chosen player / enemy gangs the Generate action
    /// regenerates from.
    ///
    /// A typed [`Resource`] (no-bare-types: every field is a named domain value). The input
    /// widgets mutate it through the named setters as a selection / commit lands;
    /// [`generate_on_press`](super::apply::generate_on_press) reads it to rebuild the model. The
    /// player / enemy gang are [`Option`] so an EMPTY [`GangRegistry`](gdtf_battle_sim::GangRegistry)
    /// (no loaded gangs) leaves them `None` — the no-gang fallback (C4): the deployment quads
    /// then label by prefab name.
    ///
    /// Declared `pub` under `test-support` (the headless C7 tests drive its fields + assert the
    /// regenerated model reflects them) and `pub(crate)` in the binary build, via
    /// [`crate::support_item!`] — the visualizer-model visibility-flip precedent.
    #[derive(Resource, Clone, PartialEq, Eq, Debug)]
    struct VizConfig {
        /// The selected theme key procgen assembles for (C1).
        theme:       ThemeUuid,
        /// The selected grid WIDTH axis (cells) — validated into a [`GridSize`] on regenerate (C2).
        width:       GridWidth,
        /// The selected grid HEIGHT axis (cells) — validated into a [`GridSize`] on regenerate (C2).
        height:      GridHeight,
        /// The selected grid LEVELS axis (storeys) — validated into a [`GridSize`] on regenerate (C2).
        levels:      GridLevels,
        /// The selected procgen seed (C3) — same seed → identical placement.
        seed:        BattleSeed,
        /// The chosen PLAYER-deployment gang (C4), or `None` for the no-gang fallback.
        player_gang: Option<GangName>,
        /// The chosen ENEMY-deployment gang (C4), or `None` for the no-gang fallback.
        enemy_gang:  Option<GangName>,
    }
}

impl VizConfig {
    /// Build the initial config from the loaded situation's `theme` + `grid_size` (C6: the same
    /// inputs the prior GTW-434 `OnEnter` build read), the [`default_viz_seed`], and no chosen
    /// gangs (the no-gang initial state). Defaulting parts come from the caller (the `OnEnter`
    /// system reads the optional [`LoadedSituation`](crate::states::LoadedSituation)).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        theme: ThemeUuid,
        grid_size: GridSize,
    ) -> Self {
        Self {
            theme,
            width: grid_size.width(),
            height: grid_size.height(),
            levels: grid_size.levels(),
            seed: default_viz_seed(),
            player_gang: None,
            enemy_gang: None,
        }
    }

    crate::support_item! {
        /// The selected theme key (C1). Used in-crate by the panel + Generate AND test-facing
        /// (the C7 theme assertion reads it through `test_support`).
        #[must_use]
        const fn theme(&self) -> ThemeUuid {
            self.theme
        }
    }

    /// Set the selected theme key (C1) — the theme-dropdown selection listener calls this.
    pub(in crate::states::running::procgen_viz) const fn set_theme(&mut self, theme: ThemeUuid) {
        self.theme = theme;
    }

    crate::support_item! {
        /// The selected seed (C3). Used in-crate by the panel + Generate AND test-facing (the
        /// C7 seed / determinism assertions read it through `test_support`).
        #[must_use]
        const fn seed(&self) -> BattleSeed {
            self.seed
        }
    }

    /// Set the selected seed (C3) — the seed-field commit listener calls this.
    pub(in crate::states::running::procgen_viz) const fn set_seed(&mut self, seed: BattleSeed) {
        self.seed = seed;
    }

    /// Set the selected grid WIDTH axis (C2) — the width-field commit listener calls this.
    pub(in crate::states::running::procgen_viz) const fn set_width(&mut self, width: GridWidth) {
        self.width = width;
    }

    /// Set the selected grid HEIGHT axis (C2) — the height-field commit listener calls this.
    pub(in crate::states::running::procgen_viz) const fn set_height(&mut self, height: GridHeight) {
        self.height = height;
    }

    /// Set the selected grid LEVELS axis (C2) — the levels-field commit listener calls this.
    pub(in crate::states::running::procgen_viz) const fn set_levels(&mut self, levels: GridLevels) {
        self.levels = levels;
    }

    crate::support_item! {
        /// The chosen PLAYER-deployment gang (C4), or `None`. Used in-crate by the panel +
        /// Generate AND test-facing (the C4 label assertion reads it through `test_support`).
        #[must_use]
        const fn player_gang(&self) -> Option<&GangName> {
            self.player_gang.as_ref()
        }
    }

    /// Set the chosen PLAYER-deployment gang (C4) — the player-gang dropdown listener calls this.
    pub(in crate::states::running::procgen_viz) fn set_player_gang(&mut self, gang: GangName) {
        self.player_gang = Some(gang);
    }

    crate::support_item! {
        /// The chosen ENEMY-deployment gang (C4), or `None`. Used in-crate by the panel +
        /// Generate AND test-facing (the C4 label assertion reads it through `test_support`).
        #[must_use]
        const fn enemy_gang(&self) -> Option<&GangName> {
            self.enemy_gang.as_ref()
        }
    }

    /// Set the chosen ENEMY-deployment gang (C4) — the enemy-gang dropdown listener calls this.
    pub(in crate::states::running::procgen_viz) fn set_enemy_gang(&mut self, gang: GangName) {
        self.enemy_gang = Some(gang);
    }

    crate::support_item! {
        /// Fold the three selected axes back into a validated [`GridSize`] (C2), or the
        /// [`GridSizeError`] naming the offending axis — the never-panicking
        /// [`GridSize::new`] path. Generate uses the `Ok` arm; the status readout uses the `Err`
        /// arm to surface an invalid combo + disable Generate. Used in-crate AND test-facing
        /// (the C7 size + invalid-size assertions read it through `test_support`).
        ///
        /// # Errors
        ///
        /// Propagates [`GridSize::new`]'s [`GridSizeError`] when any axis is zero or over its sim
        /// maximum.
        fn grid_size(&self) -> Result<GridSize, GridSizeError> {
            GridSize::new(self.width, self.height, self.levels)
        }
    }
}

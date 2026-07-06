//! The editor's **hovered-cell** model resource (GTW-512 C1.5).
//!
//! The pre-egui canvas read the hovered cell off a `bevy_ui` `Interaction::Hovered` query and the
//! QA capture WROTE `Interaction::Hovered` directly onto a cell node so the headless screenshot
//! showed the preview ghost. Under egui there are no such `bevy_ui` cell entities, so the hover is
//! lifted into a plain MODEL resource both the live egui viewport (later, C4) and the screenshot
//! capture drive write — keeping the ghost QA-able headlessly without any `bevy_ui` plumbing.
//!
//! It is a state-scoped resource (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
//! bevy-traps #1), so every reader guards with `Option<Res<…>>` / `run_if(resource_exists::<…>)`.

use bevy::prelude::*;
use gdtf_battle_sim::{metric::Level, prelude::Cell};

/// The drawable cell the cursor (or the QA capture drive) is currently hovering, if any
/// (GTW-512 C1.5).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The inner hovered slot is a domain coordinate, not a bare type
/// (no-bare-types): a ground-plane [`Cell`] paired with its storey [`Level`]. `None` means nothing
/// is hovered (the preview ghost hides). The inner field is PRIVATE (no-bare-types rule 5):
/// callers write it through [`set`](HoveredCell::set) / [`clear`](HoveredCell::clear) and read it
/// through the named accessors, so the "nothing hovered" sentinel lives in one place.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HoveredCell {
    /// The hovered `(cell, storey)` slot, or `None` when nothing is hovered.
    slot: Option<(Cell, Level)>,
}

impl HoveredCell {
    /// Build an empty hovered-cell model (nothing hovered yet) — the `OnEnter(Editing)` seed.
    #[must_use]
    pub const fn new() -> Self {
        Self { slot: None }
    }

    /// The hovered ground-plane [`Cell`], or `None` when nothing is hovered.
    #[must_use]
    pub const fn cell(&self) -> Option<Cell> {
        match self.slot {
            Some((cell, _)) => Some(cell),
            None => None,
        }
    }

    /// The hovered storey [`Level`], or `None` when nothing is hovered.
    #[must_use]
    pub const fn level(&self) -> Option<Level> {
        match self.slot {
            Some((_, level)) => Some(level),
            None => None,
        }
    }

    /// Set the hovered slot to `(cell, level)` — the live egui hover (C4) and the QA capture drive
    /// both write through here.
    pub const fn set(&mut self, cell: Cell, level: Level) {
        self.slot = Some((cell, level));
    }

    /// Clear the hovered slot (nothing hovered) — the preview ghost then hides.
    pub const fn clear(&mut self) {
        self.slot = None;
    }
}

//! The map-editor **central-canvas MODEL** — the state-scoped storey + zoom selectors the editor
//! holds while editing (GTW-423 canvas; GTW-500 selectors; egui-swept GTW-512).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The pre-egui canvas was a `bevy_ui` flex-wrapped grid of fixed-px cell nodes inside a
//! hand-rolled scroll list, with a swarm of drive systems (sync / paint / hover-ghost / centre / mouse-wheel
//! zoom / level-nav chrome). The egui swap REPLACES that whole render path with the
//! egui CENTRAL panel (the viewport — stubbed in C1, drawn in C4 / GTW-515). So the `bevy_ui` canvas
//! render machinery is GONE; this module keeps ONLY the two pure MODEL resources the editor's
//! state-scoped lifecycle inserts and the egui viewport (C4) will read:
//!
//! - [`CurrentEditLevel`] — the storey the canvas edits (the GTW-500 C1 level selector),
//! - [`CanvasZoom`] — the viewport zoom factor (the GTW-500 C3 zoom).
//!
//! Both are state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` — bevy-traps #1).

mod edit_level;
mod zoom;

pub use edit_level::{CurrentEditLevel, LevelStep};
pub use zoom::CanvasZoom;

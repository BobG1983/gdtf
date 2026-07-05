//! GTW-328 (slice 3) — the battlescape combat-text LOG (bottom-left, ABOVE the weapon panel),
//! driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down
//! to `BattleScapeState::BattleRunning`, where the real combat-log plugin spawns its container
//! and its update system drains the sim combat-event messages into fading UI text lines. They
//! cover:
//!
//! - **Lines from events** — writing the sim's `MovementOccurred` / `TurnStarted` combat-event
//!   messages makes the log gain line entities whose rendered `Text` matches the shared
//!   `classify_log_event` classifier output (the names resolved from `GangerName`).
//! - **FIFO overflow** — once more lines than the tuned `max_visible_lines` are appended, the
//!   OLDEST visible lines are despawned so the visible count is capped (the newest survive).

mod harness;
mod lines_from_events;
mod overflow;
mod presentation;
mod shot_outcomes;

//! **Playback** (GTW-727) — the presenter's own clock over the sim's act log.
//!
//! ## The problem this solves
//!
//! The sim resolves a whole exchange in one tick. One player step can provoke two
//! reaction-fire interrupts, and before this module the entire sequence — both declarations,
//! every round, both damage applications, both injuries, four combat-log lines and, if it
//! was lethal, the corpse despawn — landed inside a single frame, while the tracers were
//! still animating. The outcome was drawn before its cause: the HP had already dropped and
//! the injury was already listed while the bolt that caused them was mid-flight.
//!
//! ## The shape of the fix
//!
//! The sim NEVER waits. It writes its ordered act log and runs on. This module holds a
//! cursor into that log and consumes it at a readable pace:
//!
//! * `cursor` — how far the view has got, and what it is holding on. Every hold is bounded
//!   in wall-clock time; none depends on an entity population.
//! * `dwell` — the hot-reloadable per-deed beats, read per act at hold time.
//! * `drawn` — the `Drawn*` components: what the view is CURRENTLY SHOWING, as distinct
//!   from what the sim has already reached.
//! * `seed` — inserts those mirrors once per entity, at spawn.
//! * `emit` — `Played<M>`: a sim fact at the moment it reaches the screen. A view system
//!   that must be paced changes one word — its reader type.
//! * `apply` — showing ONE entry: its drawn writes, its `Played<M>`, and the hold it earns.
//! * `advance` — the ONE system that moves the cursor.
//! * `caught_up` — the predicate the global input gate is built on.
//! * `register` — the registration, called only from the top-down renderer plugin.
//!
//! ## Two invariants worth stating plainly
//!
//! **The dependency stays one-way.** Nothing here is named in `gdtf_battle_sim`, the sim
//! never reads a `Drawn*` component or a cursor, and no sim system waits on playback. This
//! module reads the log and draws; that is the entire relationship.
//!
//! **Nothing here can deadlock an app that has no presenter.** Every item is registered
//! only inside the top-down renderer plugin, and the catch-up predicate returns `true` when
//! the cursor or the log is absent — so a headless or autobattle app is unaffected by
//! construction rather than by a flag.
//!
//! Wiring only.

mod advance;
mod apply;
mod caught_up;
mod cursor;
mod drawn;
mod dwell;
mod emit;
mod register;
mod seed;

pub use advance::{FxPipelineProbe, advance_playback};
pub use apply::DrawnWriters;
pub use caught_up::{PlaybackGate, playback_caught_up};
pub use cursor::{ActHold, ActHoldPhase, FxSeenBusy, PlaybackCursor, SkippedActs};
pub use drawn::{DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition, DrawnVitals};
pub use dwell::{
    ConsequenceSeconds, FireBeatSeconds, ImpactCapSeconds, LifeChangeSeconds, MinorSeconds,
    PlaybackTuning, PostureSeconds, ReactionBeatSeconds, ReloadSeconds, RoundSeconds, StepSeconds,
    TurnBeatSeconds,
};
pub use emit::{Played, PlayedSignals};
pub use register::register_playback;
pub use seed::seed_drawn_state;

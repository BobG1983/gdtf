//! The buffered intent queue + the classic-intent drain system both input surfaces feed.
//!
//! # The drain invariant (GTW-571, Q5)
//!
//! The documented single-drain invariant is: **per-act generic drains in one
//! explicitly-ordered `SystemSet`, same-frame semantics preserved.** This file owns the
//! CLASSIC (non-contextual) half: every keyboard / bar / click intent is buffered onto
//! the ONE [`PendingActIntent`] queue and interpreted by the ONE
//! [`dispatch_act_intents`] drain. The CONTEXTUAL acts (Execute / Stabilize / Melee /
//! Shove / Open Door / Enter / Exit Emplacement / Throw Grenade) ride the GTW-571
//! generic machinery instead — one buffered [`PendingContextualIntents<A>`] queue and one
//! generic [`drain_contextual_intents::<A>`] per act, all in the explicitly-ordered
//! [`ContextualActSystems::Drain`] set, which runs `.before` this drain (see
//! [`crate::contextual`]). Both halves preserve the same-frame press -> `*Requested`
//! guarantee inside [`InputSystems::Gather`](crate::InputSystems).
//!
//! [`PendingContextualIntents<A>`]: crate::contextual::PendingContextualIntents
//! [`drain_contextual_intents::<A>`]: crate::contextual::drain_contextual_intents
//! [`ContextualActSystems::Drain`]: crate::contextual::ContextualActSystems

mod bundles;
mod drain;
mod queue;
mod vocabulary;

pub use bundles::{ActWriters, SelectionCycleReads};
pub use drain::dispatch_act_intents;
pub use queue::PendingActIntent;
pub use vocabulary::ActIntent;

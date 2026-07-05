//! The buffered [`PendingActIntent`] queue seam — the cross-crate write point.

use bevy::prelude::*;

use super::ActIntent;

/// The shared intent QUEUE — the buffered seam both input surfaces write.
///
/// A named newtype over a `Vec<ActIntent>` (no-bare-types: a pending-intent queue is
/// a domain value; the inner `Vec` is the collection-of-domain-values carve-out),
/// owned by `gdtf_battle_input` and made `pub` so the 222b keyboard systems AND the
/// 222c `gdtf_app` buttons both reach it across the legal `gdtf_app ->
/// gdtf_battle_input` edge. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) (its [`Default`] is the
/// empty queue) and DRAINED every update by [`dispatch_act_intents`](super::dispatch_act_intents), so a buffered
/// intent is acted on exactly once.
#[derive(Resource, Debug, Default)]
pub struct PendingActIntent(Vec<ActIntent>);

impl PendingActIntent {
    /// Queue `intent` to be drained by [`dispatch_act_intents`](super::dispatch_act_intents) next time it runs.
    ///
    /// The single write-point both surfaces call — a key system or a `gdtf_app`
    /// button system pushes the intent the press maps to. Buffered (not applied
    /// inline) so this queue's ONE drain is the only place a classic intent takes
    /// effect (the Q5 invariant: per-act generic drains in one explicitly-ordered
    /// `SystemSet`, same-frame semantics preserved — the contextual acts' per-act
    /// queues live in [`crate::contextual`]).
    pub fn push(&mut self, intent: ActIntent) {
        self.0.push(intent);
    }

    /// Take and clear every queued intent — the drain's read.
    ///
    /// Returns the buffered intents in push order and leaves the queue empty, so an
    /// intent is acted on exactly once. `pub(crate)` — only the in-crate drain
    /// consumes the queue; external surfaces only [`push`](Self::push).
    pub(crate) fn drain(&mut self) -> Vec<ActIntent> {
        core::mem::take(&mut self.0)
    }

    /// Whether the queue currently holds no intents (test/inspection helper).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

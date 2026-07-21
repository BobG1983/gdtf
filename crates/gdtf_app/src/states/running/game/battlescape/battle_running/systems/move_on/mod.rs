//! The deferred END-OF-BATTLE transition (GTW-334, extended GTW-727 C39–C41).
//!
//! The sim's victory census decides the outcome on the SAME drain frame the killing shot
//! resolves, and the run-complete latch is set that frame. That latch is the correct
//! "outcome decided" signal and is untouched. What this module owns is the separate
//! question of when it is safe to LEAVE — which is not the same instant, because the
//! deciding tracer has not flown yet and, since GTW-727, the presenter may have several
//! unplayed acts behind it.
//!
//! `phase` holds the per-run latch ([`EndTransition`]) and its bounded backstop; `system`
//! holds the gate itself.
//!
//! SCAFFOLD DIVERGENCE (GTW-575): this stays BESPOKE rather than collapsing into
//! `scaffold::advance_state_to`, because it is not a bare `NextState::set` — it classifies
//! the end, persists a phase, and holds the transition until both the FX pipeline and the
//! playback cursor have finished.
//!
//! Wiring only.

mod phase;
mod system;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape::battle_running) use phase::EndTransition;
pub(in crate::states::running::game::battlescape::battle_running) use system::move_on;

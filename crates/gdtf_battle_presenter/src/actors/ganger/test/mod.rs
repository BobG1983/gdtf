//! Unit tests for the ganger draw, split by concern (module-layout dir-form):
//! [`draw`] — the facing map, the atlas-index sum, the role table, and the per-ganger
//! tints; [`appearance`] — the GTW-631 appearance-classifier matrix (the canonical
//! stance/suppressed × Downed pins); [`resolve`] — the GTW-631 ONE-writer system
//! (change-driven both-channel stamping, the roles-change restamp, the
//! suppression-removal drain); [`visibility`] — the GTW-627 visibility classifier (the
//! band × fog compose + its band-only absent-fog branch).

mod appearance;
mod draw;
mod resolve;
mod visibility;

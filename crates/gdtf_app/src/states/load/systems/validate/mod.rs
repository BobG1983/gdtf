//! The GTW-582 **end-of-`Load` reference-integrity pass** — the ONE unified
//! dangling-reference contract (Q3 ruling): after every content family's
//! registry has resolved (and strictly before `transition_to_intro` fires, via
//! the [`ContentValidationDone`](gdtf_assets::ContentValidationDone) gate), the
//! per-edge check systems below walk the WHOLE authored content graph and
//! append a typed finding to the
//! [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport) for every
//! reference that resolves nothing; the validation-owned publish then emits ONE
//! consolidated loud report. Validation is LOUD, never fatal — it can never
//! strand `Load` (the `audit_unweighted_injuries` precedent, generalized).
//!
//! Submodules by EDGE FAMILY (wiring only here — gate directive P10, the
//! per-family checks live with their family's edge). Since GTW-630 the
//! HOST-AGNOSTIC edge checks — gangs → equipment, weapons → attachments,
//! theme/emplacement → terrain-def/weapon, and (since GTW-654) weighting rows
//! → injury keys — live in [`gdtf_content_families::validate`] beside the
//! family glue impls, so the content editor registers the SAME systems; only
//! the game-bespoke edges stay here:
//!
//! - [`register`] — the one registration surface (`add_content_validation`)
//!   plus the "every gate registry resolved" window condition.
//! - [`situation`] — the authored situation's outbound edges (gangs/members,
//!   theme, terrain, fields); app-owned (`LoadedSituation` +
//!   `SITUATION_RON_PATH` live in this crate).
//! - [`prefabs`] — prefab → theme UUID agreement and prefab → terrain-def
//!   UUIDs (a game-bespoke family the editor never loads).

mod prefabs;
mod register;
mod situation;

pub(in crate::states::load) use register::add_content_validation;

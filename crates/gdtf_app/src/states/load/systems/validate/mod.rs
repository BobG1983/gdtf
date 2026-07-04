//! The GTW-582 **end-of-`Load` reference-integrity pass** — the ONE unified
//! dangling-reference contract (Q3 ruling): after every content family's
//! registry has resolved (and strictly before `transition_to_intro` fires, via
//! the [`ContentValidationDone`](gdtf_assets::ContentValidationDone) gate), the
//! per-edge check systems below walk the WHOLE authored content graph and
//! append a typed finding to the
//! [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport) for every
//! reference that resolves nothing; the seam-owned publish then emits ONE
//! consolidated loud report. Validation is LOUD, never fatal — it can never
//! strand `Load` (the `audit_unweighted_injuries` precedent, generalized).
//!
//! Submodules by EDGE FAMILY (wiring only here — gate directive P10, the
//! per-family checks live with their family's edge):
//!
//! - [`register`] — the one registration surface (`add_content_validation`)
//!   plus the "every gate registry resolved" window condition.
//! - [`situation`] — the authored situation's outbound edges (gangs/members,
//!   theme, terrain, fields).
//! - [`gangs`] — every roster member's equipment keys (weapon / armor / melee,
//!   incl. the implicit `fists` default).
//! - [`attachments`] — every ranged + melee weapon's attachment keys (the
//!   formerly-silent resolution drop, C3(b)).
//! - [`terrain`] — theme → terrain-def UUIDs and emplacement → mounted-weapon
//!   keys.
//! - [`prefabs`] — prefab → theme UUID agreement and prefab → terrain-def
//!   UUIDs.
//! - [`injuries`] — weighting rows → injury keys (the warn-skip, now also
//!   reported through the unified pass, C3(c)).

mod attachments;
mod gangs;
mod injuries;
mod prefabs;
mod register;
mod situation;
mod terrain;

pub(in crate::states::load) use register::add_content_validation;

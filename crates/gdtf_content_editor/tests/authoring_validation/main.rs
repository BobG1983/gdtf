//! Headless pins for the editor's AUTHORING-TIME reference validation (GTW-630,
//! extended to the gangs edge in GTW-651).
//!
//! The editor app registers the GTW-582 validation pass over the edges it loads
//! (theme→terrain + emplacement→weapon since GTW-630; the gang equipment edge
//! since GTW-651 — all through the SAME `gdtf_content_families::validate`
//! checks the game registers), so a dangling key authored in the editor
//! surfaces on the [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport)
//! at authoring time, not on the next game launch. Each suite half also pins
//! the LIVE half of authoring time: a hot-edit of loaded content (the seam
//! redrive) RE-ARMS the pass — reset, re-check against the CURRENT content,
//! re-publish.
//!
//! Submodules: [`harness`] (the fixture-root editor app + publish driver +
//! report probes), [`theme`] (the GTW-630 theme→terrain pins), [`gangs`] (the
//! GTW-651 gang-equipment pins), [`save_rearm`] (the GTW-651 save-path →
//! reload → re-arm loop), [`armor_save`] (the GTW-479 armor save-path →
//! reload → re-arm loop over the gang-equipment armor edge), [`injuries_save`]
//! (the GTW-654 weighting save-path → reload → re-arm loop over the
//! weighting-row → injury-key edge).

mod armor_save;
mod gangs;
mod harness;
mod injuries_save;
mod save_rearm;
mod theme;

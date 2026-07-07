//! The editor's **authoring-time content validation** (GTW-630) — the second
//! host of the GTW-582 unified dangling-reference contract.
//!
//! The editor registers the SAME host-agnostic per-edge checks the game runs
//! ([`gdtf_content_families::validate`]) through the same seam
//! ([`ContentValidationAppExt`](gdtf_assets::ContentValidationAppExt)), for
//! exactly the edges over families the editor loads: theme → terrain-def
//! UUIDs, emplacement → mounted-weapon keys, and — since the GTW-636 Gang
//! mode brought the gang + melee-weapon registries — the gang equipment edges
//! (weapon / armor / melee keys incl. the implicit `fists` default, GTW-651).
//! The one edge whose registry the editor never loads (weapons → attachments)
//! is NOT registered — its window could never open, and a stand-in empty
//! registry would false-fail every key.
//!
//! So a dangling terrain UUID or gang equipment key authored in the editor
//! surfaces HERE, loudly, as the game's consolidated report shape — at
//! authoring time, not on the next game launch. Two halves:
//!
//! - [`register`] — the pass registration + the editor's `Check` window
//!   condition (all read registries present, not yet checked).
//! - [`rearm`] — the LIVE half: a hot-reload redrive of a watched registry
//!   re-arms the pass (reset report, re-check, re-publish), so a dangling key
//!   authored mid-session is reported at the edit, not on the next launch.

mod rearm;
mod register;

pub(crate) use register::register_validation;

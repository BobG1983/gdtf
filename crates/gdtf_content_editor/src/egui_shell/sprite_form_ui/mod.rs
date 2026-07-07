//! The SPRITE tab's egui form (GTW-664) — the DRAW half over the
//! [`sprite_form`](crate::sprite_form) model, following the gang / armor form split
//! (model module + `*_form_ui` sibling, GTW-636).
//!
//! Wiring-only module. The one-shot open-with-a-sprite seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New sprite / debug-only Save)
//! lives in [`fields`]; the CENTRAL primary panel (source picker + the visual anchor
//! affordance + facings + animation — GTW-664 C2) lives in [`panel`], composed from the
//! [`source_edit`] shared source editor, the [`preview`] anchor section (whose
//! `resolve_preview_texture` the shell calls pre-`ctx_mut`), the [`facings`] overrides
//! section, and the [`animation`] frame-row section, all over the [`cache`]'s
//! path-keyed preview/validity store.

mod animation;
mod autoload;
mod cache;
mod facings;
mod fields;
mod panel;
mod preview;
mod source_edit;

pub(crate) use autoload::autoload_first_sprite;
pub(crate) use cache::SpritePreviewCache;
pub(crate) use fields::field_stack;
pub(crate) use panel::primary_panel;
pub(crate) use preview::{PreviewTexture, resolve_preview_texture};

//! [`FocusView`] — the focus-navigable control enumeration handout (GTW-802).
//!
//! The wire mirror of "what can focus land on right now, what is focused, and what does
//! each control currently read". It rides in
//! [`AppFlowView::focus`](crate::view::AppFlowView::focus) — the lifecycle snapshot a QA
//! client polls first — so one `GetAppFlow` read answers both "where is the app" and
//! "which control can I move focus to or activate".
//!
//! Unlike the [`menu`](crate::view::menu) handout (which enumerates a menu's items), this
//! one enumerates the game's OWN focus graph: any screen that is keyboard-navigable at all
//! is enumerated here, with no marker to adopt and no per-screen wire type. A client echoes
//! a [`token`](FocusableView::token) straight back to
//! [`FocusControl`](crate::envelope::QaRequest::FocusControl) to point focus at, or
//! activate, that control.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`label`] — the scalar newtypes (caption, enabled, checked, focused).
//! - [`kind`] — the [`FocusableKindNet`] control-kind enum.
//! - [`focusable`] — the per-control [`FocusableView`] row.
//! - [`screen`] — the whole-screen [`FocusView`] snapshot.

pub mod focusable;
pub mod kind;
pub mod label;
pub mod screen;

pub use focusable::FocusableView;
pub use kind::FocusableKindNet;
pub use label::{FocusableCheckedNet, FocusableEnabledNet, FocusableLabelNet, FocusedNet};
pub use screen::FocusView;

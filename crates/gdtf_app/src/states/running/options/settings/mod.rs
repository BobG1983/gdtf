//! The typed settings model the Options screen edits (GTW-637 bsn! + widget pilot,
//! extended by GTW-868's dev-only procgen-stepper toggle).
//!
//! Wiring only (module-layout rule 2). One file per setting, plus the resource that
//! holds them:
//!
//! - [`game_settings`] — the persisted [`GameSettings`] resource.
//! - [`sound`] — the player-facing sound setting (the GTW-637 pilot's ONE setting).
//! - [`stepper`] — the DEV-ONLY procgen-stepper setting, compiled only under
//!   `dev_tools` (GTW-868), so a non-`dev_tools` build has no such setting at all.
//!
//! Every setting follows the same shape: a named newtype value (never a bare `bool`),
//! a typed newtype [`Message`](bevy::prelude::Message) intent, a field on
//! [`GameSettings`], and a `*_value_text` readout formatter shared by the spawn seed
//! and the change-driven label sync so the two never drift.

mod game_settings;
mod sound;
#[cfg(feature = "dev_tools")]
mod stepper;

pub(in crate::states::running::options) use game_settings::GameSettings;
pub(in crate::states::running::options) use sound::{
    SoundEnabled, SoundSettingChanged, sound_value_text,
};
#[cfg(feature = "dev_tools")]
pub(in crate::states::running::options) use stepper::{
    ProcgenStepperEnabled, ProcgenStepperSettingChanged, stepper_value_text,
};

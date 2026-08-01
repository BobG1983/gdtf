//! The GAME host's own wire types — the shapes its commands publish and take that the
//! shared protocol crate does not own (GTW-942, GTW-943).
//!
//! A command's arguments and reply are DERIVED from their Rust types, so any type either
//! embeds has to carry a `schemars` derive. `gdtf_qa_protocol` cannot mint these: the game's
//! five state enums are the game's, its act vocabulary mirrors the game's own input crate,
//! and a wire mirror of either belongs beside the command that publishes it rather than in a
//! crate shared with the editor.
//!
//! GTW-943 moved the pre-command id and intent vocabulary here from `gdtf_qa_protocol`
//! rather than deleting it: it is the argument vocabulary the game's later commands take,
//! and the C1 ticket completes it (the new `MouseButtonNet` and `ActSeqNet`) rather than
//! reinventing it. Nothing consumes [`act`], [`act_payload`], [`key`], [`token`],
//! [`pointer`] or [`misc`] yet — the requests that used to are gone and the commands that
//! will take them are the next tickets — so each of those `mod` declarations below carries
//! an `expect(dead_code)` naming that. The expectation stays fulfilled while any type in
//! the module is still unused, so the build fails on the stale attribute only once the LAST
//! type there has a consumer — it is a reminder to revisit the module, not a per-type alarm.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`phase`] — [`AppPhaseNet`] and the five state mirrors it is built from.
//! - [`act`] — [`NetIntent`](act::NetIntent), the mirror of the `gdtf_battle_input` act
//!   vocabulary.
//! - [`act_payload`] — the small posture / aim / melee-target payloads an act carries.
//! - [`key`] — the keyboard vocabulary the raw-input acts name.
//! - [`token`] — the entity token newtypes (`Entity::to_bits` carriers).
//! - [`pointer`] — the window pointer position.
//! - [`misc`] — the remaining scalar handles: fire-mode index, event cap, situation ref,
//!   seed, frame delay, request id.

#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod act;
#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod act_payload;
#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod key;
#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod misc;
pub(crate) mod phase;
#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod pointer;
#[expect(
    dead_code,
    reason = "the requests that used to take this vocabulary went with GTW-943's cut, and the commands that will take it are the next tickets (the game wire vocabulary, then the act / input commands). Deleting it and re-typing it later is the rewrite this epic exists to avoid. `expect` rather than `allow` so it cannot become permanent: an expectation on a `mod` stays fulfilled while ANY type inside is still unused, so it goes unfulfilled — and the build fails until the attribute is removed — once the LAST type in the module has a consumer."
)]
pub(crate) mod token;

#[cfg(test)]
mod test;

pub(crate) use phase::{
    AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
    RunningPhaseNet,
};

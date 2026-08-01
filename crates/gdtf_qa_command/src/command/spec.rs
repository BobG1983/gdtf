//! [`QaCommand`] — the typed trait one command implements.

use core::fmt::Debug;

use bevy::prelude::App;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};

/// One typed QA command a host offers.
///
/// An implementer writes a UNIT STRUCT and this impl. Everything a client sees — the name,
/// the summary, the argument schema, the reply schema, and whether the command is
/// available — is either stated here or DERIVED from the two associated types, so the
/// running app cannot advertise a shape it does not accept.
///
/// The `Debug` bound on [`Args`](Self::Args) is not decoration: a decoded call waits in a
/// [`PendingQueue`](gdtf_net_qa_transport::PendingQueue), whose deadline sweep LOGS the
/// payload of anything that timed out unclaimed, so a queued call must be printable.
///
/// This trait is deliberately NOT object-safe — [`availability`](Self::availability)
/// mentions `Self::Facts` and the two schema methods are derived from `Self::Args` /
/// `Self::Reply`. The object-safe view a catalogue and a router walk is
/// [`ErasedCommand`](super::ErasedCommand), which every `QaCommand` gets for free.
pub trait QaCommand: Sized + Send + Sync + 'static {
    /// The per-frame facts EVERY availability predicate on this host reads.
    ///
    /// One type per host, sampled once a frame by that host's router and handed to both
    /// the catalogue build and the admission check — which is how "advertised" and
    /// "admitted" are computed from one call, per command, and cannot disagree.
    type Facts: Send + Sync + 'static;

    /// The command's argument record. Its JSON Schema is derived from this type.
    type Args: DeserializeOwned + JsonSchema + Debug + Send + Sync + 'static;

    /// What the command answers with, INCLUDING its domain refusals. Its JSON Schema is
    /// derived from this type.
    ///
    /// "The act was refused because the target is not adjacent" is domain vocabulary and
    /// belongs in here, not in the envelope — which is why
    /// [`CommandOutcome`](gdtf_qa_protocol::command::CommandOutcome) has no `Refused`
    /// variant.
    type Reply: Serialize + JsonSchema + Send + Sync + 'static;

    /// The name a client runs this command by — unique within its host's set.
    ///
    /// Pinned per host by `test_support::assert_unique_names` over that host's slice (that
    /// module is behind the `test-support` feature, so it is named here rather than
    /// linked).
    const NAME: CommandName;

    /// One line telling a client what the command does and when to reach for it.
    const SUMMARY: CommandSummary;

    /// When this command answers, relative to the frame its handler claims the call.
    ///
    /// A DECLARATION, not a measurement: a handler that answers inside
    /// [`take_calls`](crate::dispatch::take_calls) says
    /// [`Immediate`](CommandTiming::Immediate), and one that parks its responder in
    /// [`DeferredReplies`](crate::dispatch::DeferredReplies) says
    /// [`Deferred`](CommandTiming::Deferred). It rides on the catalogue row so a client
    /// knows before it calls whether the answer costs it a frame or a wait.
    const TIMING: CommandTiming;

    /// Whether the command can run right now, and if not, which precondition is missing.
    ///
    /// A PURE function of the frame's facts — unit-testable with no `App`, and called by
    /// both the catalogue build and the admission check.
    fn availability(facts: &Self::Facts) -> CommandAvailability;

    /// Register the system that drains this command's queue and answers it.
    ///
    /// An ordinary Bevy system with ordinary `SystemParam`s, in whatever schedule band the
    /// command's work belongs to. The plumbing never dictates the band; it only requires
    /// that the handler runs after
    /// [`QaCommandSystems::Claim`](crate::dispatch::QaCommandSystems::Claim), which is
    /// where the decode step fills the typed queue.
    fn register_handler(app: &mut App);
}

//! [`ErasedCommand`] — the object-safe view of a [`QaCommand`], plus its blanket impl.

use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaJson,
};

use super::{QaCommand, schema::schema_text};

/// Everything the router and the catalogue need from a command WITHOUT knowing its
/// argument or reply type.
///
/// Every method returns a concrete type, so `dyn ErasedCommand<F>` exists. The argument
/// and reply types never appear here — they stay bound to the concrete `C` on the generic
/// path ([`claim_calls::<C>`](crate::dispatch::claim_calls) and the command's own handler),
/// which is what keeps dispatch typed end to end.
///
/// Do NOT implement this by hand. Outside `test_support` it has exactly one
/// implementation, the blanket `impl<C: QaCommand> ErasedCommand<C::Facts> for C` below; a
/// hand-written impl for a type that also implements [`QaCommand`] would collide with it,
/// and one for a type that does not would smuggle an entry into a host's set whose schemas
/// are not derived from any Rust type — the exact drift this whole design removes. The one
/// hand-written impl in the repo is the `FakeBrokenSchema` fixture, which is that drift on
/// purpose so `assert_schemas_parse` has something real to fail on; it is never registered
/// and never run.
///
/// # A host's list cannot hold another host's command
///
/// The blanket impl ties a command's [`QaCommand::Facts`] to the `F` of the trait object,
/// so an entry with the wrong facts type is rejected AT THE SLICE LITERAL:
///
/// ```compile_fail
/// use bevy::prelude::App;
/// use gdtf_qa_command::command::{ErasedCommand, QaCommand};
/// use gdtf_qa_protocol::command::{
///     CommandAvailability, CommandName, CommandSummary, CommandTiming,
/// };
///
/// #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
/// struct NoArgs {}
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct NoReply {}
///
/// struct HostAFacts;
/// struct HostBFacts;
///
/// struct OnHostA;
/// impl QaCommand for OnHostA {
///     type Facts = HostAFacts;
///     type Args = NoArgs;
///     type Reply = NoReply;
///     const NAME: CommandName = CommandName::from_static("host_a.only");
///     const SUMMARY: CommandSummary = CommandSummary::from_static("a host A command");
///     const TIMING: CommandTiming = CommandTiming::Immediate;
///     fn availability(_facts: &HostAFacts) -> CommandAvailability {
///         CommandAvailability::Available
///     }
///     fn register_handler(_app: &mut App) {}
/// }
///
/// // Host B's set cannot hold a host A command: `OnHostA` implements
/// // `ErasedCommand<HostAFacts>`, never `ErasedCommand<HostBFacts>`.
/// const HOST_B_COMMANDS: &[&dyn ErasedCommand<HostBFacts>] = &[&OnHostA];
/// ```
///
/// The POSITIVE counterpart is not a second doc test but real, always-compiled code: the
/// fake set in `test_support` is declared exactly this way —
/// `const FAKE_COMMANDS: &[&dyn ErasedCommand<FakeFacts>]` over commands whose `Facts` IS
/// `FakeFacts` — and every host's own list is the same shape. That the good case builds is
/// therefore observed on every run of the suite, which is what makes the failure above
/// specific rather than a compile error for some unrelated reason. (It cannot be a second
/// doc test: rustdoc merges every runnable doc test into ONE executable, and that
/// executable cannot load its dynamic libstd under the dev loop's `dynamic_linking`
/// feature, so any non-`compile_fail` block here would fail the suite for a reason that has
/// nothing to do with this crate.)
pub trait ErasedCommand<F>: Send + Sync {
    /// The command's name.
    fn name(&self) -> CommandName;
    /// The command's one-line summary.
    fn summary(&self) -> CommandSummary;
    /// When the command answers, relative to the frame its handler claims the call.
    fn timing(&self) -> CommandTiming;
    /// The JSON Schema derived from the command's argument type.
    fn arg_schema(&self) -> ArgSchemaJson;
    /// The JSON Schema derived from the command's reply type.
    fn reply_schema(&self) -> ReplySchemaJson;
    /// Whether the command can run given this frame's host facts.
    fn availability(&self, facts: &F) -> CommandAvailability;
    /// Wire this command's typed queue, its decode step, and its handler.
    ///
    /// The ONE method where a trait object re-enters the generic world: it calls
    /// [`register_command::<C>`](crate::dispatch::register_command), which knows `C`
    /// statically. That is why a host needs exactly ONE list — the same slice the
    /// catalogue walks is the slice registration walks.
    fn register(&self, app: &mut App);
}

impl<C: QaCommand> ErasedCommand<C::Facts> for C {
    fn name(&self) -> CommandName {
        C::NAME
    }

    fn summary(&self) -> CommandSummary {
        C::SUMMARY
    }

    fn timing(&self) -> CommandTiming {
        C::TIMING
    }

    fn arg_schema(&self) -> ArgSchemaJson {
        ArgSchemaJson::new(schema_text::<C::Args>())
    }

    fn reply_schema(&self) -> ReplySchemaJson {
        ReplySchemaJson::new(schema_text::<C::Reply>())
    }

    fn availability(&self, facts: &C::Facts) -> CommandAvailability {
        C::availability(facts)
    }

    fn register(&self, app: &mut App) {
        crate::dispatch::register_command::<C>(app);
    }
}

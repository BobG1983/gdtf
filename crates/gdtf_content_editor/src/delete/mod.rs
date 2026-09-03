//! Deleting an authored record, refusing while anything still references it.
//! The whole module is `#[cfg(debug_assertions)]`-gated at its `mod` site.
mod registry;
mod request;
mod systems;

pub use registry::{
    DeleteEntry, DeleteRegistry, RemoveRecordFile, RestoreRecord, TakeRecord, TakenRecord,
};
pub use request::{DeleteOutcome, DeleteRefusal, DeleteRequest};
pub(crate) use systems::register_delete;

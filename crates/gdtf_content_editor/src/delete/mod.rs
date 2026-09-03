//! Deleting an authored record, refusing while anything still references it.
//! The whole module is `#[cfg(debug_assertions)]`-gated at its `mod` site.
mod entries;
mod offered;
mod registry;
mod request;
mod systems;

#[cfg(test)]
mod test;

pub(crate) use offered::entries_offered_on;
pub use registry::{
    DeleteEntry, DeleteRegistry, DeleteScreen, RecordFilePath, RestoreRecord, TakeRecord,
    TakenRecord, take_matching_assets,
};
pub use request::{DeleteOutcome, DeleteRefusal, DeleteRequest};
pub(crate) use systems::register_delete;

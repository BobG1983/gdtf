//! Deleting an authored record, rewriting or dropping every reference to it first.
//! The whole module is `#[cfg(debug_assertions)]`-gated at its `mod` site.
mod control;
mod entries;
mod offer;
mod offered;
mod registry;
mod request;
mod resolution;
mod settle;
mod systems;

#[cfg(test)]
mod test;

pub(crate) use control::delete_control;
pub use offer::{
    OfferResolution, ReplacementCandidate, ReplacementLabel, ReplacementOffer, labelled_candidates,
};
pub(crate) use offered::entries_offered_on;
pub use registry::{
    DeleteEntry, DeleteRegistry, DeleteScreen, RecordCandidates, RecordFilePath, RestoreRecord,
    TakeRecord, TakenRecord, take_matching_assets,
};
pub use request::{DeleteOutcome, DeleteRefusal, DeleteRequest};
pub use resolution::{
    DropReferences, DroppedReferences, ReferenceResolution, ReplaceReferences, ReplacementCheck,
};
pub(crate) use systems::register_delete;

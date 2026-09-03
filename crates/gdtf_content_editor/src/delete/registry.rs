//! The families a delete can remove a record from, and how each one does it.

use std::any::Any;

use bevy::prelude::{Resource, World};
use gdtf_assets::{ContentMemberKey, FindingFamily};

/// A record taken out of its registry, held while the delete waits.
pub type TakenRecord = Box<dyn Any + Send + Sync>;

/// Takes the record under a key out of its registry.
pub type TakeRecord =
    Box<dyn Fn(&mut World, &ContentMemberKey) -> Option<TakenRecord> + Send + Sync + 'static>;

/// Puts a taken record back under its key.
pub type RestoreRecord =
    Box<dyn Fn(&mut World, &ContentMemberKey, TakenRecord) + Send + Sync + 'static>;

/// Removes the file the record under a key was read from.
pub type RemoveRecordFile =
    Box<dyn Fn(&mut World, &ContentMemberKey) -> std::io::Result<()> + Send + Sync + 'static>;

/// One deletable content family: the label its findings carry, and the three
/// operations a delete needs.
pub struct DeleteEntry {
    family:      FindingFamily,
    take:        TakeRecord,
    restore:     RestoreRecord,
    remove_file: RemoveRecordFile,
}

impl DeleteEntry {
    /// Name a deletable family and the operations that delete one of its records.
    #[must_use]
    pub const fn new(
        family: FindingFamily,
        take: TakeRecord,
        restore: RestoreRecord,
        remove_file: RemoveRecordFile,
    ) -> Self {
        Self {
            family,
            take,
            restore,
            remove_file,
        }
    }

    /// The finding family label this entry deletes.
    #[must_use]
    pub const fn family(&self) -> &FindingFamily {
        &self.family
    }

    /// Take the record under `key` out of its registry.
    pub fn take(&self, world: &mut World, key: &ContentMemberKey) -> Option<TakenRecord> {
        (self.take)(world, key)
    }

    /// Put a taken record back under `key`.
    pub fn restore(&self, world: &mut World, key: &ContentMemberKey, record: TakenRecord) {
        (self.restore)(world, key, record);
    }

    /// Remove the file the record under `key` was read from.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error if the path cannot be removed.
    pub fn remove_file(&self, world: &mut World, key: &ContentMemberKey) -> std::io::Result<()> {
        (self.remove_file)(world, key)
    }
}

/// Every content family the editor can delete a record from.
#[derive(Resource, Default)]
pub struct DeleteRegistry(Vec<DeleteEntry>);

impl DeleteRegistry {
    /// True when this registry holds an entry for `family`.
    #[must_use]
    pub fn handles(&self, family: &FindingFamily) -> bool {
        self.entry(family).is_some()
    }

    /// Make one more family deletable.
    pub fn add(&mut self, entry: DeleteEntry) {
        self.0.push(entry);
    }

    /// The entry for `family`, if this registry holds one.
    #[must_use]
    pub fn entry(&self, family: &FindingFamily) -> Option<&DeleteEntry> {
        self.0.iter().find(|entry| **entry.family() == **family)
    }
}

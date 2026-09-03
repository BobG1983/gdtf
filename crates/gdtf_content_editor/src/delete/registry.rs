//! The families a delete can remove a record from, and how each one does it.

use std::any::Any;

use bevy::{
    asset::{Asset, Assets},
    prelude::{Resource, World},
};
use gdtf_assets::{ContentMemberKey, ContentSourcePath, FindingFamily};

use crate::mode::{EditorMode, InjurySubTab};

/// A record taken out of its registry, held while the delete waits.
pub type TakenRecord = Box<dyn Any + Send + Sync>;

/// Takes the record under a key out of its registry.
pub type TakeRecord =
    Box<dyn Fn(&mut World, &ContentMemberKey) -> Option<TakenRecord> + Send + Sync + 'static>;

/// Puts a taken record back under its key.
pub type RestoreRecord =
    Box<dyn Fn(&mut World, &ContentMemberKey, TakenRecord) + Send + Sync + 'static>;

/// Answers the asset-root-relative file the record under a key was read from.
pub type RecordFilePath =
    Box<dyn Fn(&World, &ContentMemberKey) -> Option<ContentSourcePath> + Send + Sync + 'static>;

/// Take every asset the predicate matches out of the store, returning them in registration order.
#[must_use]
pub fn take_matching_assets<T: Asset>(
    assets: &mut Assets<T>,
    matches: impl Fn(&T) -> bool,
) -> Vec<T> {
    let ids: Vec<_> = assets
        .iter()
        .filter(|(_id, asset)| matches(asset))
        .map(|(id, _asset)| id)
        .collect();
    let mut taken = Vec::new();
    for id in ids {
        if let Some(asset) = assets.remove(id) {
            taken.push(asset);
        }
    }
    taken
}

/// Where the editor offers one delete: a mode tab, and a sub-tab inside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DeleteScreen {
    mode:    EditorMode,
    sub_tab: Option<InjurySubTab>,
}

impl DeleteScreen {
    /// Name the mode tab, and the sub-tab inside it when the mode has one.
    #[must_use]
    pub const fn new(mode: EditorMode, sub_tab: Option<InjurySubTab>) -> Self {
        Self { mode, sub_tab }
    }

    /// Whether this screen is the one the editor has open.
    #[must_use]
    pub fn is_open(&self, mode: EditorMode, sub_tab: Option<InjurySubTab>) -> bool {
        self.mode == mode && self.sub_tab.is_none_or(|wanted| Some(wanted) == sub_tab)
    }
}

/// One deletable content family: the label its findings carry, the screen that
/// offers it, and the operations a delete needs.
pub struct DeleteEntry {
    family:   FindingFamily,
    screen:   DeleteScreen,
    take:     TakeRecord,
    restore:  RestoreRecord,
    relative: RecordFilePath,
}

impl DeleteEntry {
    /// Name a deletable family, where the editor offers it, and how one record goes.
    #[must_use]
    pub const fn new(
        family: FindingFamily,
        screen: DeleteScreen,
        take: TakeRecord,
        restore: RestoreRecord,
        relative: RecordFilePath,
    ) -> Self {
        Self {
            family,
            screen,
            take,
            restore,
            relative,
        }
    }

    /// The finding family label this entry deletes.
    #[must_use]
    pub const fn family(&self) -> &FindingFamily {
        &self.family
    }

    /// The screen the editor offers this delete on.
    #[must_use]
    pub const fn screen(&self) -> &DeleteScreen {
        &self.screen
    }

    /// Take the record under `key` out of its registry.
    pub fn take(&self, world: &mut World, key: &ContentMemberKey) -> Option<TakenRecord> {
        (self.take)(world, key)
    }

    /// Put a taken record back under `key`.
    pub fn restore(&self, world: &mut World, key: &ContentMemberKey, record: TakenRecord) {
        (self.restore)(world, key, record);
    }

    /// The asset-root-relative file the record under `key` was read from.
    #[must_use]
    pub fn relative_path(
        &self,
        world: &World,
        key: &ContentMemberKey,
    ) -> Option<ContentSourcePath> {
        (self.relative)(world, key)
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

    /// The finding family label of every entry this registry holds.
    pub fn labels(&self) -> impl Iterator<Item = &FindingFamily> {
        self.0.iter().map(DeleteEntry::family)
    }

    /// Every entry this registry holds, in registration order.
    pub(crate) fn entries(&self) -> impl Iterator<Item = &DeleteEntry> {
        self.0.iter()
    }
}

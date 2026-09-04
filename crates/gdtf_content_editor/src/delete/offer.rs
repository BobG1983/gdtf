//! The replacement a delete offers the author when a reference has to be kept.

use std::collections::HashMap;

use bevy::prelude::Resource;
use gdtf_assets::ContentMemberKey;

/// One record the author can point a kept reference at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementCandidate {
    key:   ContentMemberKey,
    label: ReplacementLabel,
}

impl ReplacementCandidate {
    /// Name one candidate by its registry key and the row an author reads.
    #[must_use]
    pub const fn new(key: ContentMemberKey, label: ReplacementLabel) -> Self {
        Self { key, label }
    }

    /// The key the author writes into [`ReplacementOffer::choose`] to pick this one.
    #[must_use]
    pub const fn key(&self) -> &ContentMemberKey {
        &self.key
    }

    /// The row an author reads for this candidate.
    #[must_use]
    pub const fn label(&self) -> &ReplacementLabel {
        &self.label
    }
}

/// The row an author reads for one replacement candidate.
#[derive(bevy::prelude::Deref, Debug, Clone, PartialEq, Eq)]
pub struct ReplacementLabel(String);

impl ReplacementLabel {
    /// Wrap a candidate row.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// What the author did with the offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferResolution {
    /// Go ahead with whatever `choose` holds.
    Confirm,
    /// Abandon the delete, writing nothing and removing nothing.
    Cancel,
}

/// The open replacement offer: what can be picked, what was picked, and whether to go on.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ReplacementOffer {
    candidates:  Vec<ReplacementCandidate>,
    /// The candidate the author picked, if any.
    pub choose:  Option<ContentMemberKey>,
    /// Whether the author confirmed or cancelled; the delete waits while this is `None`.
    pub resolve: Option<OfferResolution>,
}

impl ReplacementOffer {
    /// Open an offer over the records that could stand in for the one being deleted.
    #[must_use]
    pub const fn new(candidates: Vec<ReplacementCandidate>) -> Self {
        Self {
            candidates,
            choose: None,
            resolve: None,
        }
    }

    /// The records that could stand in for the one being deleted.
    #[must_use]
    pub fn candidates(&self) -> &[ReplacementCandidate] {
        &self.candidates
    }

    /// Whether the offer holds a candidate under `key`.
    #[must_use]
    pub fn offers(&self, key: &ContentMemberKey) -> bool {
        self.candidates
            .iter()
            .any(|candidate| candidate.key() == key)
    }
}

/// Candidate rows from key and label pairs, sorted by label.
///
/// A label two or more records share carries that record's key in brackets.
#[must_use]
pub fn labelled_candidates(
    named: impl IntoIterator<Item = (ContentMemberKey, ReplacementLabel)>,
) -> Vec<ReplacementCandidate> {
    let mut rows: Vec<(ContentMemberKey, ReplacementLabel)> = named.into_iter().collect();
    let mut held: HashMap<&str, usize> = HashMap::new();
    for (_key, label) in &rows {
        *held.entry(label.as_str()).or_insert(0) += 1;
    }
    let shared: Vec<String> = held
        .into_iter()
        .filter(|(_label, count)| *count > 1)
        .map(|(label, _count)| label.to_owned())
        .collect();
    rows.sort_by(|left, right| {
        (*left.1)
            .cmp(&*right.1)
            .then_with(|| (*left.0).cmp(&*right.0))
    });
    rows.into_iter()
        .map(|(key, label)| {
            let row = if shared.contains(&*label) {
                ReplacementLabel::new(format!("{}  [{}]", *label, *key))
            } else {
                label
            };
            ReplacementCandidate::new(key, row)
        })
        .collect()
}

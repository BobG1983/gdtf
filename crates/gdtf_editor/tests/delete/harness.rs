//! How far this suite advances a delete, and how a case answers the offer it opens.

use std::path::Path;

use bevy::prelude::App;
use gdtf_assets::{ContentMemberKey, ContentValidationDone, FindingFamily};
use gdtf_editor::{
    DeleteOutcome, DeleteRequest, EditorMcpAssetsRoot, OfferResolution, ReplacementOffer,
};

use crate::{advance::advance_to_published, app::editor_app_with_asset_root};

/// The finding family label every ranged weapon check writes.
pub(crate) const WEAPON_FAMILY: &str = "WeaponRegistry";

/// The finding family label every terrain piece check writes.
pub(crate) const TERRAIN_FAMILY: &str = "TerrainDefRegistry";

/// The finding family label every theme check writes.
pub(crate) const THEME_FAMILY: &str = "UuidThemeRegistry";

/// The finding family label every gang check writes.
pub(crate) const GANG_FAMILY: &str = "GangRegistry";

/// An editor app on `root`, saving under it, advanced until its report is published.
pub(crate) fn editor_on(root: &Path) -> App {
    let mut app = editor_app_with_asset_root(root);
    app.insert_resource(EditorMcpAssetsRoot::new(root.to_path_buf()));
    advance_to_published(&mut app);
    app
}

/// Ask for one delete and run it out, answering any offer it opens with `answer`.
pub(crate) fn run_delete(
    app: &mut App,
    family: &str,
    key: &str,
    answer: &OfferAnswer,
) -> SettledDelete {
    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(family.to_owned()),
        ContentMemberKey::new(key.to_owned()),
    ));
    advance_answering(app, answer)
}

/// The member key a replacement is chosen by.
#[must_use]
pub(crate) fn member_key(key: &str) -> ContentMemberKey {
    ContentMemberKey::new(key.to_owned())
}

/// Enough updates for a delete to settle, and few enough to fail rather than hang.
pub(crate) const OUTCOME_UPDATES: usize = 16;

/// How a case answers the replacement offer, when the delete opens one.
pub(crate) enum OfferAnswer {
    /// Confirm, naming this replacement, or naming none when it is `None`.
    Confirm(Option<ContentMemberKey>),
    /// Call the delete off.
    Cancel,
    /// Answer nothing at all, for a case that expects no offer to open.
    Nothing,
}

/// What the bounded run saw: the outcome if one landed, and whether an offer opened.
pub(crate) struct SettledDelete {
    /// The outcome the delete settled on, absent when it never settled.
    pub(crate) outcome: Option<DeleteOutcome>,
    /// Whether a [`ReplacementOffer`] was open on any of the updates the run made.
    pub(crate) offered: bool,
}

/// Run at most [`OUTCOME_UPDATES`] updates, answering the outcome if one landed.
pub(crate) fn advance_to_outcome(app: &mut App) -> Option<DeleteOutcome> {
    advance_answering(app, &OfferAnswer::Nothing).outcome
}

/// Run at most [`OUTCOME_UPDATES`] updates, answering any offer that opens with `answer`.
pub(crate) fn advance_answering(app: &mut App, answer: &OfferAnswer) -> SettledDelete {
    let mut offered = false;
    for _ in 0..OUTCOME_UPDATES {
        app.update();
        if app.world().get_resource::<ReplacementOffer>().is_some() {
            offered = true;
            answer_offer(app, answer);
        }
        if let Some(outcome) = app.world().get_resource::<DeleteOutcome>() {
            return SettledDelete {
                outcome: Some(outcome.clone()),
                offered,
            };
        }
    }
    SettledDelete {
        outcome: None,
        offered,
    }
}

// Write one answer into the open offer, the way the shell's own control would.
fn answer_offer(app: &mut App, answer: &OfferAnswer) {
    let Some(mut offer) = app.world_mut().get_resource_mut::<ReplacementOffer>() else {
        return;
    };
    match answer {
        OfferAnswer::Confirm(choose) => {
            offer.choose.clone_from(choose);
            offer.resolve = Some(OfferResolution::Confirm);
        }
        OfferAnswer::Cancel => offer.resolve = Some(OfferResolution::Cancel),
        OfferAnswer::Nothing => {}
    }
}

/// Whether validation had republished by the time the advance gave up.
pub(crate) fn is_published(app: &App) -> bool {
    app.world()
        .get_resource::<ContentValidationDone>()
        .is_some()
}

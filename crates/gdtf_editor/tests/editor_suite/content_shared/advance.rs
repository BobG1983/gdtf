//! Running the editor app until its integrity report has been published.

use bevy::prelude::App;
use cobalt_test_utils::advance_until;
use gdtf_assets::ContentValidationDone;

/// Run updates until the integrity report has been published.
pub(crate) fn advance_to_published(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some()
    });
    for _ in 0..4 {
        app.update();
    }
}

//! The Stance control's press → intent mapping and its sim-driven active-segment sync
//! (GTW-267 / GTW-277).
//!
//! GTW-277 migrated the Stance control to a `gdtf_ui` vertical
//! [`SegmentedControl`](gdtf_ui::SegmentedControl) (see
//! [`spawn_stance_panel`](super::stance_panel::spawn_stance_panel)). Selecting a segment
//! pushes a DIRECT [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance) for the
//! named posture (NOT a blind cycle), exactly as the three ad-hoc toggles did — so the
//! `prone_toggle_sets_stance_prone_directly` byte-equal parity is preserved. The active
//! mark is the widget's own [`ActiveSegment`](gdtf_ui::ActiveSegment) highlight, synced FROM
//! the selected ganger's [`Stance`](gdtf_battle_sim::Stance).

use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{Stance, StanceKind};
use gdtf_ui::{ActiveSegment, SegmentSelected, SegmentedControl};

use super::stance_panel::{stance_for_index, stance_index};
use crate::states::running::game::battlescape::action_bar::components::StanceControl;

/// Pushes a DIRECT [`ActIntent::SetStance`] when a Stance segment is selected by the user
/// (GTW-267 / GTW-277).
///
/// Reads [`SegmentSelected`](gdtf_ui::SegmentSelected) messages (emitted by `gdtf_ui`'s
/// [`select_segment_on_press`](gdtf_ui::select_segment_on_press) on a real click), and for
/// each whose control carries the [`StanceControl`] marker, maps the chosen
/// [`SegmentIndex`](gdtf_ui::SegmentIndex) → [`StanceKind`] and
/// [`push`](PendingActIntent::push)es the SAME
/// [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance)`(kind)` the direct
/// stance intent (and the keyboard stance keys' `SetStance` path) pushes — so the widget +
/// key surfaces stay parallel over the ONE [`PendingActIntent`] drain (the byte-equal parity
/// `prone_toggle_sets_stance_prone_directly` pins). The keyboard stance-CYCLE key still
/// pushes the cycling [`ActIntent::StanceCycle`] (unchanged). A select with no selection is a
/// no-op in the drain (AC6).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageReader<SegmentSelected>`](MessageReader)
/// (bevy-traps rule 4), the [`ResMut<PendingActIntent>`](ResMut) write, and a read-only
/// `Query<(), With<StanceControl>>` — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn stance_segment_intent(
    mut chosen: MessageReader<SegmentSelected>,
    mut pending: ResMut<PendingActIntent>,
    stance_controls: Query<(), With<StanceControl>>,
) {
    for event in chosen.read() {
        if stance_controls.get(event.control).is_err() {
            continue;
        }
        if let Some(kind) = stance_for_index(*event.index) {
            pending.push(ActIntent::SetStance(kind));
        }
    }
}

/// Drives the Stance control's active SEGMENT from the selected ganger's
/// [`Stance`](gdtf_battle_sim::Stance) (GTW-267 / GTW-277).
///
/// Reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter); if it holds an entity
/// with a [`Stance`], the control root's [`ActiveSegment`](gdtf_ui::ActiveSegment) is set to
/// that posture's index ([`stance_index`]) — so `gdtf_ui`'s
/// [`repaint_segments`](gdtf_ui::repaint_segments) highlights exactly that segment (filled +
/// bold) and the others return to base the same frame (mutually-exclusive, the color-blind-
/// safe active mark). With no selection — or a selected entity carrying no [`Stance`] — the
/// active segment is left UNCHANGED (the highlight holds its last posture rather than
/// flickering to none, since the control always has a valid active index). Written via
/// [`set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) so it repaints only on a real
/// change. No despawn/respawn — pure index write ([[ui-mutate-not-respawn]]).
///
/// Param-only (`bevy-traps.md` #7): a `Res<SelectedShooter>` read, a read-only
/// `Query<&Stance>`, and a `Query<&mut ActiveSegment, With<StanceControl>>` write — no
/// `&mut World`.
pub(in crate::states::running::game::battlescape) fn sync_stance_active_segment(
    selected: Res<SelectedShooter>,
    stances: Query<&Stance>,
    mut controls: Query<&mut ActiveSegment, (With<StanceControl>, With<SegmentedControl>)>,
) {
    // The selected ganger's posture, if any (no selection / no `Stance` → leave the
    // highlight where it is; the control always carries a valid active index).
    let current: Option<StanceKind> = (**selected)
        .and_then(|entity| stances.get(entity).ok())
        .map(|stance| **stance);
    let Some(kind) = current else {
        return;
    };
    let want = ActiveSegment::new(stance_index(kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}

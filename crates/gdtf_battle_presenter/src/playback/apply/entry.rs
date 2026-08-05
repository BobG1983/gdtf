//! Apply one act-log entry to drawn state and emit its played signals.

use gdtf_battle_sim::act_log::ActEntry;

use super::{
    super::{cursor::ActHold, dwell::PlaybackTuning, emit::PlayedSignals},
    hold::hold_for,
    mirror::mirror_drawn,
    signal::emit_played,
    writers::DrawnWriters,
};

pub(in crate::playback) fn show_entry(
    entry: &ActEntry,
    tuning: &PlaybackTuning,
    drawn: &mut DrawnWriters,
    played: &mut PlayedSignals,
) -> ActHold {
    mirror_drawn(entry, drawn);
    emit_played(entry, played);
    hold_for(entry, tuning)
}

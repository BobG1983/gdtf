//! How long the cursor dwells on each kind of act-log entry.

use gdtf_battle_sim::act_log::{ActDeed, ActEntry};

use super::super::{cursor::ActHold, dwell::PlaybackTuning};

pub(super) fn hold_for(entry: &ActEntry, tuning: &PlaybackTuning) -> ActHold {
    match entry.deed() {
        ActDeed::TurnBegan { .. } => ActHold::timed(*tuning.turn_beat_seconds),
        ActDeed::PostureChanged { .. } => ActHold::timed(*tuning.posture_seconds),
        ActDeed::Stepped { .. } | ActDeed::MovedTo { .. } => ActHold::timed(*tuning.step_seconds),
        ActDeed::Fired { .. } if entry.provenance().is_reaction() => {
            ActHold::timed(*tuning.reaction_beat_seconds)
        }
        ActDeed::Fired { .. } => ActHold::timed(*tuning.fire_beat_seconds),
        ActDeed::RoundResolved { .. } => {
            ActHold::awaiting_impact(*tuning.impact_cap_seconds, *tuning.round_seconds)
        }
        ActDeed::Reloaded { .. } => ActHold::timed(*tuning.reload_seconds),
        ActDeed::LifeChanged { .. } => ActHold::timed(*tuning.life_change_seconds),
        ActDeed::Injured { .. }
        | ActDeed::VitalsChanged { .. }
        | ActDeed::Fell { .. }
        | ActDeed::Struck { .. }
        | ActDeed::DiedAt { .. }
        | ActDeed::Suppressed { .. }
        | ActDeed::ArmorBroke { .. }
        | ActDeed::DotStarted { .. }
        | ActDeed::FieldStarted { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => ActHold::timed(*tuning.consequence_seconds),
        ActDeed::MoveRefused { .. }
        | ActDeed::EnteredView { .. }
        | ActDeed::MagazineChanged { .. }
        | ActDeed::DotTicked { .. }
        | ActDeed::FieldTicked { .. }
        | ActDeed::BleedStarted
        | ActDeed::Bled
        | ActDeed::TerrainPieceSmashed { .. } => ActHold::timed(*tuning.minor_seconds),
    }
}

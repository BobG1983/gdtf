mod families;
mod log_event;
mod palette;
mod pop;
mod reader;
mod slot_allocator;
mod stacked_reader;
mod text;

#[cfg(test)]
mod test;

pub use families::{
    ArmorBrokenFct, BleedingFct, DotFct, FieldFct, InjuryFct, OnDeathFct, SuppressionFct,
};
pub use log_event::{
    CombatLogEvent, CombatLogSource, CombatLogSourceAppExt, CombatLogSystems, InjuryLogText,
    LogLine, LogName, classify_log_event, forward_live_log_source, forward_log_source,
    forward_turn_started,
};
pub use palette::{FctValence, severity_color, valence_color};
pub use pop::{ConsequenceFct, ConsequencePop, PopAnchor};
pub(super) use reader::{ClassifiedPop, anchor_cell, classify_report};
pub use slot_allocator::{FctAnchorCell, FctSlotAllocator};
pub use stacked_reader::{
    ConsequenceFctAppExt, ConsequenceFctSystems, read_consequence_fct,
    register_consequence_fct_core,
};
pub use text::{
    CombatText, FctEmphasis, FctStackIndex, FloatingCombatText, animate_floating_text,
    spawn_floating_text,
};

//! Handle end-turn requests: advance faction, regen TU, emit [`TurnStarted`].

use bevy::prelude::{Message, MessageReader, MessageWriter, Query, ResMut};

use crate::{
    acts::EndTurnRequested,
    ganger::{Faction, Tu, TuMax},
    turn::{ActiveFaction, regen::regen_team_tu},
};

/// A new faction's turn has begun.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurnStarted {
    /// Faction that is now active.
    pub now_active: Faction,
}

impl TurnStarted {
    /// Build a turn-started message.
    #[must_use]
    pub const fn new(now_active: Faction) -> Self {
        Self { now_active }
    }
}

/// On [`EndTurnRequested`], advance active faction, regen their TU, emit [`TurnStarted`].
pub fn dispatch_end_turn(
    mut requests: MessageReader<EndTurnRequested>,
    mut active: ResMut<ActiveFaction>,
    mut gangers: Query<(&Faction, &mut Tu, &TuMax)>,
    mut turns: MessageWriter<TurnStarted>,
) {
    for _request in requests.read() {
        active.advance();
        let now_active = **active;
        regen_team_tu(
            gangers
                .iter_mut()
                .map(|(faction, tu, tu_max)| (faction, tu.into_inner(), tu_max)),
            now_active,
        );
        turns.write(TurnStarted::new(now_active));
    }
}

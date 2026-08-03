use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

#[derive(Message, Deref, Debug, Clone)]
pub struct Played<M: Message + Clone>(M);

impl<M: Message + Clone> Played<M> {
        #[must_use]
    pub const fn new(fact: M) -> Self {
        Self(fact)
    }
}

#[derive(SystemParam)]
pub struct PlayedSignals<'w> {
        pub(super) turn:          MessageWriter<'w, Played<TurnStarted>>,
        pub(super) step:          MessageWriter<'w, Played<MovementOccurred>>,
        pub(super) refusal:       MessageWriter<'w, Played<MoveRejected>>,
        pub(super) declaration:   MessageWriter<'w, Played<FireDeclaration>>,
        pub(super) round:         MessageWriter<'w, Played<ShotFired>>,
        pub(super) reload:        MessageWriter<'w, Played<ReloadResult>>,
        pub(super) injury:        MessageWriter<'w, Played<InjuryInflicted>>,
        pub(super) fall:          MessageWriter<'w, Played<FallOccurred>>,
        pub(super) strike:        MessageWriter<'w, Played<MeleeStruck>>,
        pub(super) death:         MessageWriter<'w, Played<OnDeathOccurred>>,
        pub(super) suppression:   MessageWriter<'w, Played<SuppressionApplied>>,
        pub(super) armor_broken:  MessageWriter<'w, Played<ArmorBroken>>,
        pub(super) dot:           MessageWriter<'w, Played<DotAfflicted>>,
        pub(super) dot_tick:      MessageWriter<'w, Played<DotTicked>>,
        pub(super) field:         MessageWriter<'w, Played<FieldAfflicted>>,
        pub(super) field_tick:    MessageWriter<'w, Played<FieldTicked>>,
        pub(super) bleed_started: MessageWriter<'w, Played<BleedStarted>>,
        pub(super) bleeding:      MessageWriter<'w, Played<Bleeding>>,
        pub(super) cover:         MessageWriter<'w, Played<CoverDestroyed>>,
        pub(super) melee_landed:  MessageWriter<'w, Played<MeleeResolved>>,
        pub(super) throw_landed:  MessageWriter<'w, Played<ThrowResolved>>,
}

pub(super) fn register_played_messages(app: &mut App) {
    app.add_message::<Played<TurnStarted>>()
        .add_message::<Played<MovementOccurred>>()
        .add_message::<Played<MoveRejected>>()
        .add_message::<Played<FireDeclaration>>()
        .add_message::<Played<ShotFired>>()
        .add_message::<Played<ReloadResult>>()
        .add_message::<Played<InjuryInflicted>>()
        .add_message::<Played<FallOccurred>>()
        .add_message::<Played<MeleeStruck>>()
        .add_message::<Played<OnDeathOccurred>>()
        .add_message::<Played<SuppressionApplied>>()
        .add_message::<Played<ArmorBroken>>()
        .add_message::<Played<DotAfflicted>>()
        .add_message::<Played<DotTicked>>()
        .add_message::<Played<FieldAfflicted>>()
        .add_message::<Played<FieldTicked>>()
        .add_message::<Played<BleedStarted>>()
        .add_message::<Played<Bleeding>>()
        .add_message::<Played<CoverDestroyed>>()
        .add_message::<Played<MeleeResolved>>()
        .add_message::<Played<ThrowResolved>>();
}

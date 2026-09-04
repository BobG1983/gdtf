use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::Keybinds;
use serde::{Deserialize, Serialize};

use super::keys::{key_message, resolve};
use crate::dev::mcp::{
    facts::GameFacts,
    wire::key::{KeyNet, KeyPressNet},
};

/// The refusal a named action gets on a host with no keybind table installed.
const NO_KEYBINDS: RefusalNote = RefusalNote::from_static(
    "a named action is resolved through the `Keybinds` resource, and this host has no keybind \
     table installed — press the physical key instead",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputPressKeyArgs {
    /// Physical key, or a named action resolved through the live keybind table.
    key: KeyPressNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InputPressKeyReply {
    /// Physical key the press was delivered as, after any rebind was resolved.
    key: KeyNet,
}

pub(crate) struct InputPressKey;

impl QaCommand for InputPressKey {
    type Args = InputPressKeyArgs;
    type Facts = GameFacts;
    type Parked = KeyNet;
    type Reply = InputPressKeyReply;

    const NAME: CommandName = CommandName::from_static("input.press_key");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Press a key the way the window manager does: the call writes the same KeyboardInput \
         message bevy_winit writes, so every consumer of ButtonInput sees it on the next frame. \
         A named action is resolved through the live keybind table, so a rebound key is honoured \
         and the reply names the physical key that was actually pressed. The reply is held back \
         until the frame that read the press has finished and the release has been written, so \
         the caller's next command sees both the effect of the press and a settled keyboard.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Last,
            (release_input_press_key, claim_input_press_key).chain(),
        );
    }
}

fn claim_input_press_key(
    mut queue: ResMut<PendingQueue<CommandCall<InputPressKey>>>,
    mut deferred: ResMut<DeferredReplies<InputPressKey>>,
    keybinds: Option<Res<Keybinds>>,
    mut keyboard: MessageWriter<KeyboardInput>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<InputPressKey>(&mut queue) {
        let Some(key) = resolve(args.key, keybinds.as_deref()) else {
            responder.unavailable(UnavailableCode::MissingModel, NO_KEYBINDS);
            continue;
        };
        keyboard.write(key_message(key, ButtonState::Pressed));
        deferred.park(responder, key);
    }
}

fn release_input_press_key(
    mut deferred: ResMut<DeferredReplies<InputPressKey>>,
    mut keyboard: MessageWriter<KeyboardInput>,
) {
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_resolved(|key| {
        keyboard.write(key_message(*key, ButtonState::Released));
        Some(InputPressKeyReply { key: *key })
    });
    debug!(
        delivered = *delivered,
        "mcp: input.press_key released the keys it pressed a frame ago"
    );
}

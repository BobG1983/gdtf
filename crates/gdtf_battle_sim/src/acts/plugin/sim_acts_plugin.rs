use bevy::prelude::{App, Plugin};

#[derive(Debug, Default, Clone, Copy)]
pub struct SimActsPlugin;
impl Plugin for SimActsPlugin {
    fn build(&self, app: &mut App) {
        super::messages::register_messages(app);
        super::acts::wire_acts(app);
        super::turn_clocks::wire_turn_clocks(app);
        super::reaction_suppression::wire_reaction_suppression(app);
    }
}

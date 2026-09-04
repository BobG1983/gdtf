use bevy::prelude::*;

crate::support_item! {
    /// Inserted once the situation has finished generating.
    #[derive(Resource)]
    pub(crate) struct GenerationComplete;
}

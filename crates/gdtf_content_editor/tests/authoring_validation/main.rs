//! Headless pins for the editor's AUTHORING-TIME reference validation (GTW-630,
//! checks the game registers), so a dangling key authored in the editor
//! at authoring time, not on the next game launch. Each suite half also pins
//! the LIVE half of authoring time: a hot-edit of loaded content (the hot-reload
mod armor_save;
mod attachments;
mod gangs;
mod harness;
mod injuries_save;
mod save_rearm;
mod sprites;
mod theme;

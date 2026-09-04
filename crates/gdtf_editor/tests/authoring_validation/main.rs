//! Headless pins for editor authoring-time reference validation.
//! Covers registry checks and hot-edit revalidation after a save.
#[path = "../content_shared/advance.rs"]
mod advance;
#[path = "../content_shared/app.rs"]
mod app;
mod armor_save;
mod attachments;
#[path = "../content_shared/findings.rs"]
mod findings;
mod gangs;
mod harness;
mod injuries_save;
mod on_death;
mod prefabs;
mod save_rearm;
mod situation;
mod sprites;
mod terrain_views;
mod theme;

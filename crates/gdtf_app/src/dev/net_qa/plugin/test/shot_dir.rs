use bevy::prelude::*;
use gdtf_screenshot::ShotDir;

use crate::dev::net_qa::plugin::register_consumers::register_consumers;

#[test]
fn the_qa_shot_directory_is_absolute_so_captures_leave_no_stray_target() {
    let mut app = App::new();
    register_consumers(&mut app);

    let Some(dir) = app.world().get_resource::<ShotDir>() else {
        unreachable!("registering the QA consumers must insert a shot directory");
    };
    assert!(
        dir.is_absolute(),
        "a relative shot directory writes a stray `target/` under whatever directory the process \
         runs from; got {}",
        dir.display(),
    );
    assert!(
        !dir.components().any(|part| part.as_os_str() == ".."),
        "the shot directory must not carry a `..` segment: {}",
        dir.display(),
    );
    assert!(
        dir.ends_with("target/qa_screenshots"),
        "QA captures belong under the workspace `target/`, which the root `.gitignore` covers; \
         got {}",
        dir.display(),
    );
}

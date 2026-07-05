//! GTW-289 REGRESSION TEST: clicking an enemy produces a SHOT end-to-end.
//!
//! GTW-289 ("clicking an enemy = no shot") was found ALREADY FIXED on re-test — the original
//! cause was the GTW-297 empty-weapon-registry black-screen, since fixed. This test guards the
//! fix: it wires the full click->shot chain so a future regression that re-silences the fire
//! path fails here.
//!
//! The existing `acts.rs::left_click_emits_one_fire_requested` proves the INPUT half of the
//! chain (a left-click on an enemy emits exactly one `FireRequested`), but stops at the
//! message — its shooter is UNARMED (no `Weapon` marker, no battle surfaces), so `fire()` is a
//! no-op and no shot is observable. This test wires the WHOLE chain:
//!
//! ```text
//!   left-click (real cursor -> InspectTarget)            [gdtf_battle_input]
//!     -> left_click_act decides FIRE                   (decision.rs)
//!       -> ActIntent::Fire pushed                      (apply_left_click)
//!         -> dispatch_act_intents drains -> FireRequested emitted   (intent/seam.rs)
//!           -> dispatch_fire gates (arc) + runs fire() (acts/fire.rs, SimActsPlugin)
//!             -> observable: shooter TU debited, magazine decremented  (fire/volley.rs)
//! ```
//!
//! It spawns a FULLY-ARMED shooter (the `ShooterQuery` `With<Weapon>` set + the `TargetQuery`
//! battle surfaces the shooter's own liveness reads through) and a battle-surface ENEMY in the
//! occupancy grid, places them in a LEGITIMATE firing situation (in range, in-arc, alive,
//! loaded, affordable), synthesizes the click, runs one `update()`, and asserts the shot ran by
//! the SAME seed-agnostic relation the sim's own AC3 test uses: the shooter's TU strictly
//! dropped (the up-front mode charge) — hit/miss is seed-dependent, the CHARGE is not.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out (a)); no
//! function here takes `&mut World`/`&World`.

mod fire_shot;
mod harness;

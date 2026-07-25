---
name: "ADR 0003: Hand-rolled, data-driven UI on first-party bevy_ui"
description: The player-facing game UI is built on first-party bevy_ui, themed from RON via a typed GdtfTheme resource, with loose assets and a headless-tested hot-reload policy. Amended 2026-07-05 (GTW-635) to PREFER Bevy's first-party widget primitives (bevy_ui_widgets) and author new screens with bsn!, and again 2026-07-25 (GTW-865) to settle the stack allocation: hand-rolled bevy_ui for the game, egui for the content editor and dev_tools surfaces, with neither stack crossing into the other's side.
---

# 0003. Hand-rolled, data-driven UI on first-party `bevy_ui`

## Status

`Accepted` — 2026-06-13. Driven by the main-menu UI block (GTW-112 / GTW-113 /
GTW-114) and the loose-asset decision recorded under GTW-117.

`Amended` — 2026-07-05 (GTW-635), per USER RULING adopting the game-UI-stack
research recommendation. See [Amendment — 2026-07-05](#amendment--2026-07-05-gtw-635)
below: first-party widget primitives are now PREFERRED over hand-rolling, new
screens are `bsn!`-authored, and `egui` is used for dev-only surfaces. The
data-driven-theming, one-way-crate-graph, loose-asset, and hot-reload clauses are
unchanged.

`Amended` — 2026-07-25 (GTW-865), per USER RULING settling which UI stack owns
which surface. See [Amendment — 2026-07-25](#amendment--2026-07-25-gtw-865)
below: the player-facing game UI is hand-rolled `bevy_ui`, the content editor is
`egui`, neither crosses into the other, and `dev_tools`-gated `egui` stays. This
amendment REVOKES the conditional form-heavy-meta-screen `egui` fallback added by
the 2026-07-05 amendment. The data-driven-theming, one-way-crate-graph,
loose-asset, and hot-reload clauses remain unchanged. This ADR stays `Accepted` —
it is NOT superseded.

## Context

GDTF is entering its first UI work: the main-menu block (GTW-112 / GTW-113 /
GTW-114) that rebuilds the *grimdark* menu in Bevy. Before any of those tickets
lands, the UI/theme posture for the *whole* block needs to be fixed once, so
every menu ticket is built against the same binding contract rather than
re-litigating dependencies, theming, and asset strategy per ticket.

The forces at play:

- **Pillar 5 — readability over fidelity**
  ([../pillars/5-readability-over-fidelity.md](../pillars/5-readability-over-fidelity.md)).
  GDTF competes on legible consequence, not production values: a 2D tactical
  presentation and text-forward reports. The UI does not need a heavyweight
  styling framework; it needs total legibility and a grimdark typeset feel.
- **Dependency hygiene.** Bevy's ecosystem has UI/styling crates (`iyes_*`,
  `bevy-ui-*`, and similar), but each is a churning third-party surface pinned
  to a Bevy minor version. Pulling one in trades a small authoring convenience
  for a recurring upgrade-tax and a supply-chain surface on the menu — the most
  visible, least combat-critical part of the app.
- **The crate graph is the enforcement mechanism.** ADR 0001 established the
  one-way model/view dependency and noted it is now *enforceable by the crate
  graph*. UI is another layer that must not be allowed to reach sideways into
  the app or the sim, and the cleanest way to guarantee that is the same
  cargo-level fence.
- **Theme values change often; recompiling to retheme is friction.** A grimdark
  palette and type scale are tuning data, not logic. Hardcoding them as `const
  Color`s couples every visual tweak to a rebuild and scatters the source of
  truth across modules.
- **Assets and packaging are at different lifecycles.** The menu needs a font
  (`Alegreya-Variable.ttf`) and a theme file *now*; how the shipped game bundles
  its assets is a packaging concern that does not need to be solved to build the
  menu.

## Decision

We will build the main-menu UI block as **hand-rolled UI on first-party
`bevy_ui` only**, themed from data, with loose assets and a headless-tested
hot-reload policy. Specifically:

1. **Hand-rolled on first-party `bevy_ui` ONLY.** The UI is composed directly
   from Bevy's own `bevy_ui` node/widget primitives. We will NOT take any
   Bevy-ecosystem UI or styling crate — no `iyes_*`, no `bevy-ui-*`, no
   third-party widget/styling/layout framework. A change that adds such a
   dependency violates this ADR.
   (AMENDED 2026-07-05, GTW-635 — see the
   [Amendment](#amendment--2026-07-05-gtw-635): Bevy's own first-party widget
   primitives in `bevy_ui_widgets` are now PREFERRED over hand-rolling. The
   third-party-ecosystem ban this clause exists for is unchanged —
   `bevy_ui_widgets` is FIRST-PARTY Bevy, not a Bevy-ecosystem crate.)
2. **Ordinary Rust crates are permitted.** This ban is on *Bevy-ecosystem UI*
   crates, not on general-purpose libraries. Plain Rust crates the UI needs —
   e.g. `ron` for deserialization, `serde` for the derive — ARE allowed and
   expected.
3. **A one-way crate graph, enforced by cargo.** The UI lives in its own crate,
   `gdtf_ui`, which depends on **bevy only**. It must NEVER depend on
   `gdtf_app`, `gdtf_battle_sim`, or `gdtf_battle_presenter`. The app composes
   the UI crate; the UI crate never reaches back. This extends the one-way
   discipline of ADR 0001 to the UI layer, enforced by the crate graph rather
   than by convention.
4. **Data-driven theming.** Theme VALUES — colours, the type scale, spacing — are
   authored in RON at `assets/core_tuning/ui_theme.tuning.ron` and deserialized into a typed
   theme schema. They are NOT hardcoded as `const Color`s in Rust. The resolved
   theme is a single `GdtfTheme` **Resource** that is the one styling source of
   truth, read by a central `apply_theme` system; styling flows from that
   resource outward, never from scattered literals.
5. **Loose assets now; embedding deferred to packaging.** All assets — the font
   `Alegreya-Variable.ttf` and the theme RON — ship as **loose files** under the
   repo-root `assets/` directory and are loaded through Bevy's `AssetServer`. We
   will NOT use `embedded_asset!` or any asset-embedding mechanism anywhere at
   this stage. Embedding is **deferred to packaging time** as a later, separate
   decision (this is why the earlier embed-first framing carried by GTW-117 is
   dropped). Recorded as a user decision on 2026-06-13.
6. **Hot-reload with a headless-tested policy.** Live theme reapply — re-running
   `apply_theme` when the theme asset changes — ships in the first menu. The OS
   file-watcher that detects on-disk edits is fenced behind a **local/dev binary
   feature** and is **NOT** exercised in CI. The reapply LOGIC is verified
   **headlessly** by injecting an `AssetEvent::Modified` for the theme asset and
   asserting the resource and styling update; in Bevy 0.19 an `AssetEvent` is a
   `MessageReader` MESSAGE (not `EventReader`), so the test drives it as a
   message. Visual confirmation of a live retheme is **local-only**.

## Amendment — 2026-07-05 (GTW-635)

`Amended` — 2026-07-05, per USER RULING adopting the game-UI-stack research
recommendation. The original decision predates two things maturing in Bevy: the
first-party widget primitives that now ship in `bevy_ui_widgets` (part of the
`bevy` `ui` feature), and the `bsn!` scene macro hardening into the proven
action-bar authoring pattern. The built battlescape HUD is now bevy_ui + `bsn!` —
six of its nine panels and the main menu are already `bsn!`-authored. The posture
is refined as follows; everything else in this ADR (data-driven theming, the
one-way `gdtf_ui` crate graph, loose assets, and the headless-tested hot-reload
policy) is UNCHANGED.

1. **First-party primitives preferred; hand-roll only the gaps.** "Hand-rolled on
   first-party `bevy_ui` ONLY" (Decision clause 1) becomes: build on `bevy_ui`,
   and PREFER Bevy's own first-party widget primitives — the `EditableText` text
   widget (`bevy_text`, driven by `bevy_ui_widgets`'s `EditableTextInputPlugin`),
   `bevy_ui_widgets`'s `MenuButton`, `ScrollArea`, and their siblings —
   over hand-rolling. Hand-roll ONLY what upstream lacks. The widgets Bevy does
   not provide stay hand-rolled in `gdtf_ui`: `ProgressBar`, `Pips`, `Switch`, and
   `SegmentedControl`. This does NOT relax the third-party ban — `bevy_ui_widgets`
   is FIRST-PARTY Bevy, not a Bevy-ecosystem crate, so clause 1's real target
   (`iyes_*` / `bevy-ui-*` / any third-party widget-styling-layout framework)
   still stands.
2. **New screens are `bsn!`-authored.** Every NEW screen is authored with the
   `bsn!` scene macro, following the proven action-bar pattern already used by the
   built HUD. GTW-637 (the `bsn!` Options-screen pilot) is the pilot that LOCKS
   this policy if it holds on authoring velocity — or BREAKS it if it does not
   (see clause 3).
   (AMENDED 2026-07-25, GTW-865 — see the
   [Amendment](#amendment--2026-07-25-gtw-865): the `bsn!` policy is LOCKED and
   its conditional escape hatch is removed, because the clause 3 fallback it
   pointed at is revoked. Every new player-facing screen is `bsn!`-authored.)
3. **`egui` is dev-only, plus a pre-approved meta-screen fallback.** `egui` (via
   `bevy_egui`) is the stack for DEV-ONLY surfaces — the content editor
   is the established precedent. It is ALSO the pre-approved fallback for NEW
   form-heavy META screens, but ONLY if the GTW-637 `bsn!` Options pilot fails on
   authoring velocity. It is NEVER used for the built battlescape HUD.
   (AMENDED 2026-07-25, GTW-865 — see the
   [Amendment](#amendment--2026-07-25-gtw-865): the dev-only grant is CONFIRMED
   and sharpened — the content editor is now the settled allocation, not merely a
   precedent, and `dev_tools`-gated `egui` stays permitted. The pre-approved
   form-heavy-meta-screen fallback is REVOKED: its GTW-637 condition will never
   be evaluated. The closing "NEVER used for the built battlescape HUD" sentence
   is confirmed and WIDENED to the whole player-facing game UI.)

## Amendment — 2026-07-25 (GTW-865)

`Amended` — 2026-07-25, per USER RULING settling the UI-stack allocation. The
2026-07-05 amendment left the question half-open: `egui` was the dev-only stack
*plus* a conditional fallback for new form-heavy meta screens, with the GTW-637
`bsn!` Options pilot as the deciding test. A UI-stack comparison programme
(GTW-796 and its subtree) was then opened to settle it with measurements. That
programme was **abandoned before producing any**: no comparison demo was built,
no authoring-velocity or maintenance numbers were collected, and the whole
comparison subtree — both demo epics, their 20 children, the five shared
prerequisites, and the GTW-813 ratification — was Canceled. This amendment records a decision made on
the user's judgement, not on collected evidence. It also REVERSES the 2026-07-24
"one UI stack, no dev-only exemption" ruling recorded on GTW-822, which was
conditional on a hand-rolled win that was never demonstrated.

A note on process, so a future reader does not read this as a rule violation to
"fix": [the index rules](index.md) say an `Accepted` ADR is immutable and is
replaced by a superseding ADR, not edited. The user explicitly authorised
amending this `Accepted` ADR in place for this instance (2026-07-25 — "I'm
overruling the ADR rule for this instance", confirming "Amend 0003 in place").
That is a **one-time exception**, not a policy change: the index rules are
unchanged and still govern the next ADR. This ADR remains `Accepted`; ADR number
`0008` is still free for a genuinely new decision.

The posture is refined as follows; everything else in this ADR (data-driven
theming, the one-way `gdtf_ui` crate graph, loose assets, and the
headless-tested hot-reload policy) is UNCHANGED.

1. **The allocation is settled.** `egui` (via `bevy_egui`) is the stack for the
   **content editor**. Hand-rolled `bevy_ui` — with `bsn!` authoring and Bevy's
   first-party `bevy_ui_widgets` primitives, per the 2026-07-05 amendment — is
   the stack for the **player-facing game**: the battlescape HUD and the meta
   screens. This is no longer a precedent or a default; it is the allocation.
2. **The boundary runs in BOTH directions.** No `gdtf_ui` or hand-rolled
   `bevy_ui` in the content editor. No `egui` in the player-facing game UI. Two
   stacks are acceptable ONLY while that boundary holds — the boundary is the
   reason two stacks are tolerable at all, so a change that mixes them is a
   change to this ADR, not a local choice.
3. **`dev_tools`-gated `egui` remains permitted.** The rule in clause 2 targets
   the player-facing game UI, not every dev affordance.
   [`crates/gdtf_app/src/dev/procgen_stepper/`](../../crates/gdtf_app/src/dev/procgen_stepper/)
   is the live example and stays. The 2026-07-05 amendment's cap holds: a
   release binary never links `egui`.
4. **The conditional meta-screen fallback is REVOKED.** The 2026-07-05
   amendment's clause 3 offered `egui` as a pre-approved fallback for new
   form-heavy meta screens *if* the GTW-637 `bsn!` Options pilot failed on
   authoring velocity. That condition will never be evaluated — the allocation
   is settled by ruling instead — so the fallback is retired outright rather
   than left dangling. Its companion sentence ("NEVER used for the built
   battlescape HUD") is confirmed and WIDENED to the whole player-facing game
   UI by clause 2 above.
5. **`bsn!`-authoring is LOCKED.** The 2026-07-05 amendment's clause 2 hung the
   `bsn!` policy on the same GTW-637 pilot, with an escape hatch pointing at the
   fallback clause 4 has now revoked. With that fallback gone the escape hatch
   points at nothing, so the policy stands without a condition: every new
   player-facing screen is `bsn!`-authored.
6. **Gamepad support bears on the allocation.** The game UI must support gamepad
   FOCUS NAVIGATION and ACTIVATION; gamepad POINTER emulation is permanently out.
   The content editor is exempt from the gamepad requirement. This is recorded
   only because it constrains the allocation — a stack for the game UI has to be
   drivable by focus movement rather than by a cursor. The input design itself is
   not this ADR's subject.

### Consequences of this amendment

- **No UI code change was required.** The shipped state already matched the
  allocation: the game UI is `bevy_ui` + `bsn!`, the editor is `bevy_egui`, and
  neither mixes. This amendment records reality rather than directing work.
- **The comparison scaffolding was removed.** The coexistence proof and the
  stack-swap harness built to run the abandoned comparison — their two `dev`
  modules under `gdtf_app`, their test suites, the wire swap intent and its
  `BattleView` field, the `F9` shortcut, and the `bevy_egui` dev-dependency
  they needed — were still in the tree at the time of this amendment; GTW-864
  deleted them. `EguiPlugin` ownership moved to the existing dev-affordances
  plugin, so the `dev_tools` procgen stepper clause 3 preserves still gets its
  egui context. The comparison notes under
  `docs/ui-stack-comparison/` are removed by GTW-866.
- **The boundary is a documented convention, not a mechanical check.** The user
  deliberately declined a conformance test in favour of comments on the affected
  `Cargo.toml` manifests (GTW-863). A crossing dependency will be caught by
  review against this ADR, not by a failing build.

## Consequences

- **Pillar 5 is served directly.** Effort goes into a legible, grimdark-typeset
  menu and the data that themes it, not into wiring and maintaining a styling
  framework GDTF does not need. The presentation stays as light as the pillar
  asks.
- **Dependency surface stays minimal.** The menu carries no third-party UI crate
  and no version-pinned styling framework to chase across Bevy upgrades. The
  only new third-party surface is general-purpose data crates (`ron`, `serde`).
- **The UI boundary is enforceable, not aspirational.** Because `gdtf_ui`
  depends on bevy only, a cargo build fails the moment the UI tries to reach into
  the app or the sim. The one-way discipline of ADR 0001 now covers the UI layer
  by construction.
- **Retheming is a data edit, not a recompile.** Colours and the type scale live
  in `assets/core_tuning/ui_theme.tuning.ron` behind the typed `GdtfTheme` resource; tuning
  the look is editing data, and with hot-reload it is observable live in a dev
  build. The single-source-of-truth resource keeps styling legible and central.
- **Packaging is left open on purpose.** Loose assets keep iteration simple now;
  a future ADR decides embedding/bundling at packaging time. Until then, the
  shipped layout assumes a loose `assets/` directory next to the binary.
- **Hot-reload is split: logic is gated, watching is fenced.** The reapply path
  is covered by a deterministic headless test (injected `AssetEvent::Modified`),
  so CI proves the logic without an OS file-watcher; the watcher itself is a
  dev-only feature whose visual effect is confirmed locally. A future change that
  wants watcher behavior in CI would have to revisit this.
- **Cost / constraints created.** Hand-rolling means we author widget and layout
  code that an ecosystem crate might have provided, and we own the typed theme
  schema and its RON. Any later desire for an ecosystem UI crate, asset
  embedding, or CI-exercised file-watching must supersede the corresponding
  clause of this ADR rather than slip in silently.

## Alternatives considered

- **Adopt a Bevy-ecosystem UI/styling crate (`iyes_*`, `bevy-ui-*`, etc.).**
  Rejected. It would buy authoring convenience at the cost of a churning,
  version-pinned third-party surface on the least combat-critical, most visible
  part of the app — exactly the dependency-hygiene cost this ADR exists to
  avoid. Pillar 5 means the menu does not need a styling framework's weight.
- **Hardcode theme values as `const Color`s in Rust.** Rejected. It couples
  every visual tweak to a recompile, scatters the styling source of truth across
  modules, and forecloses hot-reload. Data-driven RON behind a single
  `GdtfTheme` resource keeps the look legible, central, and tunable.
- **Let `gdtf_ui` depend on `gdtf_app` (or the sim/presenter) for convenience.**
  Rejected. It would dissolve the boundary this ADR is establishing and
  reintroduce exactly the sideways coupling ADR 0001's crate-graph discipline
  forecloses. The UI reads nothing from the app or the sim; the app composes the
  UI.
- **Embed assets now via `embedded_asset!`.** Rejected for this stage. Embedding
  is a packaging concern at a different lifecycle than building the menu; loose
  files under `assets/` keep dev iteration and hot-reload simple. Embedding is
  deferred to a packaging-time decision rather than baked in early (superseding
  the earlier GTW-117 embed framing).
- **Exercise the OS file-watcher in CI.** Rejected. A live file-watcher makes CI
  depend on OS filesystem-event timing — flaky and slow. Injecting
  `AssetEvent::Modified` headlessly verifies the reapply logic deterministically;
  the watcher stays a dev-only feature confirmed locally.

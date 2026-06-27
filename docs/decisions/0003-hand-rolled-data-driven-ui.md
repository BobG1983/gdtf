---
name: "ADR 0003: Hand-rolled, data-driven UI on first-party bevy_ui"
description: The main-menu UI block is hand-rolled on first-party bevy_ui only, themed from RON via a typed GdtfTheme resource, with loose assets and a headless-tested hot-reload policy.
---

# 0003. Hand-rolled, data-driven UI on first-party `bevy_ui`

## Status

`Accepted` — 2026-06-13. Driven by the main-menu UI block (GTW-112 / GTW-113 /
GTW-114) and the loose-asset decision recorded under GTW-117.

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

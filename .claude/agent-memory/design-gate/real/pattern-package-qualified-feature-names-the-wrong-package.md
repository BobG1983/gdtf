---
name: pattern-package-qualified-feature-names-the-wrong-package
description: A `pkg/feature` flag resolves by PACKAGE name, not by binary-target name — prose claiming a CI or alias flag enables a bin's own passthrough feature is false whenever the bin package is named differently from the library.
metadata:
  type: feedback
---

Whenever a diff adds prose about which features CI or the `.cargo/config.toml` aliases turn
on, read `^name =` in every manifest that prose touches and match it against the exact
`pkg/feature` strings in the commands. `pkg/feature` resolves by PACKAGE name. A binary
target's name is not its package's name, and in this workspace they deliberately differ.

**Why:** `bins/gdtf_content_editor/Cargo.toml:2` declares the package
`gdtf_content_editor_bin`, while `:7` names its binary target `gdtf_content_editor` — the
same string as the library package at `crates/gdtf_content_editor/Cargo.toml:2`. Three names,
two identical, and only the package name is the one a `pkg/feature` flag addresses. So a
comment in the bin's manifest claiming that some `--features gdtf_content_editor/<f>` command
turns on that file's own passthrough feature (`:15-16`) is false: the flag reaches the
library, and the bin's feature stays off. The consequence sentence such a comment usually
carries can still be true — the library module IS covered — which is what makes the false
half easy to wave through.

`.cargo/config.toml:17-19` and `:22` use this syntax for real
(`grimdark_turfwar/dynamic_linking`, `grimdark_turfwar/dev_tools`), so the trap is live, and
a manifest comment is prose that nothing lints.

**How to apply:** grep `^name =` across the manifests, then diff that list against the
package names actually written in `.github/workflows/*.yml` and `.cargo/config.toml`. Report
a claim about feature reach as a violation whenever the package name in the flag is not the
package whose feature the prose says is enabled.

Related: [[pattern-manifest-dep-defeats-flag-guard]],
[[pattern-uncontracted-prose-asserts-false-history]].

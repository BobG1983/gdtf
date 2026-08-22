---
paths: 
  - "**/*.rs"
  - "**/*.ron"
---

# Comment hygiene

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

- '///' / '//!' are for public API documentation. Use them for all public items.
- '//' is for private implementation notes. Use them for all private items.
- '///' / '//!' rule: Max 2 lines. State what the item is and what it does. Then stop.
- '//' rule: Max 1 line. Only non-obvious implementation notes. Prefer no comment.
- No "GTW-n" (or any ticket ID) in any comment.
- No design rationale in any comment. Design rationale belongs in the ticket, not in code.
- Prefer deleting a comment over rewriting it badly.
- Comments **MUST** follow Plain language rules: see [plain-language.md](./plain-language.md).

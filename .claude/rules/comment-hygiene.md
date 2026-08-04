---
paths: 
  - "**/*.rs"
  - "**/*.ron"
---

# Comment hygiene

- '///' / '//!' are for public API documentation. Use them for all public items.
- '//' is for private implementation notes. Use them for all private items.
- '///' / '//!' rule: Max 2 lines. State what the item is and what it does. Then stop.
- '//' rule: Only non-obvious implementation notes. Prefer no comment.
- No "GTW-n" (or any ticket ID) in any comment.
- No design rationale in any comment. Design rationale belongs in the ticket, not in code.
- Prefer deleting a comment over rewriting it badly.
- Plain language rules still apply: see [plain-language.md](./plain-language.md).

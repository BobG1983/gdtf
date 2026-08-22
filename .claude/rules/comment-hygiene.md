---
paths:
  - "**/*.rs"
  - "**/*.ron"
---

# Comment hygiene

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

- Use `///` and `//!` on all public items.
- Use `//` on all private items.
- Keep a `///` or `//!` to 2 lines. Say what the item is and what it does.
- Keep a `//` to 1 line. Write one only where the implementation is not obvious. Prefer
  no comment.
- Never put a ticket ID such as `GTW-n` in a comment.
- Design rationale belongs on the ticket, never in a comment.
- Delete a comment rather than rewrite it badly.

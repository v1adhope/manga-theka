---
name: audit-pr
description: "Audit the current PR against its spec and tickets, post the findings as a PR comment, and bring its description up to date."
disable-model-invocation: true
---

Audit PR. Find where code stray from spec and from tickets PR close.

Read PR. Read linked issues. Read diff vs main branch. Read project standards.

Tag every acceptance criterion and standard: met, deviated, deferred to other ticket, or bundled outside scope. Cite code. Verify first, then claim. Code no back it? Drop it.

Sum up as checklist, group by issue, deviations first, each one carry fix or owning ticket. Nothing violated? Say so plain.

Show draft. Then post as comment on PR.

Then update PR description with what audit found.

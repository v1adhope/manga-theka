# AGENTS.md

This is an API for reading and hosting user-uploaded manga, manhwa, and manhua.

## General

- Adhere to ISO, RFC, NIST, and IEEE technical and governance standards where applicable to the task, citing the specific standard when relevant.
- Keep secrets out of this repository.
- Ask before running any destructive or hard-to-reverse command.
- Use simple ASCII punctuation in file edits, unless the file format requires otherwise.
- Do not edit `CHANGELOG.md` unless explicitly asked.

## Plan Mode

- Make the plan extremely concise. Sacrifice grammar for the sake of concision.
- At the end of each plan, give me a list of unresolved questions to answer, if any.

## Commands

- Refer to `taskfile.yml` for executable Bash commands. Run `task --list` to check available.

## Documentation hierarchy

| File                          | Covers                                                                         |
| ----------------------------- | ------------------------------------------------------------------------------ |
| `docs/rust.md`                | Rust coding standards, style, and patterns.                                    |
| `docs/behavior-guidelines.md` | Behavioral guidelines for coding assistance: assumptions, scope, verification. |
| `docs/git.md`                 | Version control, branching, and commit rules.                                  |
| `docs/issue-tracker.md`       | Issue tracking conventions, lifecycle, and PR linkage.                         |
| `docs/sql.md`                 | SQL coding standards, query guidelines, and schema conventions.                |
| `docs/security.md`            | Security practices, checks, and rationale.                                     |
| `docs/domain.md`              | How to consume this repo's domain documentation when exploring.                |

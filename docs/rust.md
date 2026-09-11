# Rust context

## Conventions

- Don't write unsafe code.
- Router-contract enums serialize their variants verbatim — no `rename_all`, no per-variant `rename`.
- Refer to `docs/api.md` for wire casing and time format.
- Follow Guard Clause / Early Return Pattern.
- Log internal errors immediately at the point of failure.
- Don't pass external invalid data to the application layer, use `NewType Pattern`.
- Derive what you use now — add macros when a real need arises, not in advance for possible future use. Exception is `Debug`.
- Don't add indirection for a single implementation — no trait or generic parameter with one implementor. Add it when the second one exists, or when a layer boundary needs the dependency inverted.
- Avoid chained conversions like `value.as_ref().to_string()` — implement the target conversion directly on value's type instead of composing it from intermediate ones.
- Traits such as `From`, `TryFrom`, `Display`, and `Debug` represent structural behavior and syntax guarantees, not execution side-effects.
- Flag if expected to use raw identifier syntax like `r#type`.
- Declare a constant at the smallest scope that covers all its uses — function, file, or module, in that order of preference.
- Keep one canonical entity per responsibility in a domain layer — a command entity and a read/query entity — instead of operation-specific aliases for each use case. A single entity may serve both responsibilities when the read and command shapes coincide; split only when they diverge.
- A repository owns its own transaction boundary by default. Put begin/commit in the application layer when the atomic unit spans more than one repository or depends on a business decision; in that case give the repository methods involved a `&mut PgConnection` parameter so they can join the caller's transaction.
- Don't add comments (`//` or `///`) unless explicitly asked. When asked, a comment answers *why*, not *what*; if it restates the code, flag it.
- Service layer methods take ownership of the entity they act on, even when only forwarding `&item` downstream.

## Dependencies & Versioning

- Refer to `../Cargo.toml` for actual crate versions and active dependencies.
- Prefer the latest major version for new crates; fallback to the latest minor if unavailable.

## Project structure

All code must strictly adhere to Clean Architecture principles with explicit layer separation and unidirectional dependency flow:

- **Domain Layer**: Contains core business entities, value objects, and domain logic.
- **Application Layer**: Defines business use cases, application services.
- **Infrastructure Layer**: Handles all external technical concerns, including database persistence, API integrations, framework code, and hardware.
- **Controllers Layer**: Handles API endpoints, request parsing, input validation, HTTP status codes, and response serialization by delegating to Application use cases.

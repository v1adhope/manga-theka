# rust.md

## Conventions

- Don't write unsafe code.
- Use `camelCase` for JSON keys and `PascalCase` for enum JSON values in router contracts.
- Follow Guard Clause / Early Return Pattern.
- Log internal errors immediately at the point of failure.
- Time format is `RFC3339`.
- Don't pass external invalid data to the application layer, use `NewType Pattern`.
- Derive what you use now — add macros when a real need arises, not in advance for possible future use. Exception is `Debug`.
- Avoid chained conversions like `value.as_ref().to_string()` — implement the target conversion directly on value's type instead of composing it from intermediate ones.
- Traits such as `From`, `TryFrom`, `Display`, and `Debug` represent structural behavior and syntax guarantees, not execution side-effects.
- Flag if expected to use raw identifier syntax like `r#type`.

## Dependencies & Versioning

- Refer to `../Cargo.toml` for actual crate versions and active dependencies.
- Prefer the latest major version for new crates; fallback to the latest minor if unavailable.

## Project structure

All code must strictly adhere to Clean Architecture principles with explicit layer separation and unidirectional dependency flow:

- **Domain Layer**: Contains core business entities, value objects, and domain logic.
- **Application Layer**: Defines business use cases, application services.
- **Infrastructure Layer**: Handles all external technical concerns, including database persistence, API integrations, framework code, and hardware.
- **Controllers Layer**: Handles API endpoints, request parsing, input validation, HTTP status codes, and response serialization by delegating to Application use cases.

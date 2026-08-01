# Env-only configuration via envious

The application reads its runtime configuration (server address and database connection) exclusively from process environment variables, parsed through the `envious` crate with the `APP_` prefix and `__` (double-underscore) field separator. Required variables cause a panic at startup; no defaults are provided; no `.env` file is loaded by the binary itself. A `.env.example` is committed to the repository as a reference only.

Hand-rolled parsing was rejected to avoid bespoke handling of nested fields and to keep the public contract for env-var names explicit and discoverable; `figment` / TOML overlays were rejected because the project has six flat values and no per-environment differences yet, and adding the machinery speculatively would outpace the need. If the project later gains nested config (e.g. `server.tls.*`) or per-environment profiles, `figment` with a TOML base + env overlay becomes the natural next step.

# Env-only configuration via envious

The application reads all runtime configuration exclusively from environment variables via the `envious` crate, `APP_` prefix, `__` field separator. Required variables panic at startup; no defaults; no `.env` file loaded by the binary. `.env.example` is committed as a reference only.

Hand-rolled parsing was rejected to avoid bespoke nested-field handling and keep env-var names explicit and discoverable. `figment`/TOML overlays were rejected: the config is shallow -- one flat level plus a nested struct per dependency -- with no per-environment differences yet, so the machinery would outpace the need. Revisit on a change in *shape*, not size: nesting past one level (e.g. `database.pool.*`), or the first value that must differ per environment. Until then, `figment` with a TOML base + env overlay is the recorded next step, not the current one.

This governs the mechanism, not the inventory. Adding a value or a nested group is a change to `Config` (`src/config.rs`) and `.env.example` -- together the authoritative list -- not an amendment to this ADR.

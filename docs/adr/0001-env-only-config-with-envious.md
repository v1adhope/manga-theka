# Runtime configuration comes only from environment variables, via `envious`

All runtime config is read from env vars through the `envious` crate: `APP_` prefix, `__` field separator. Required variables panic at startup; no defaults, no `.env` file loaded by the binary. `.env.example` is committed as reference only.

**Rejected.** Hand-rolled parsing -- to avoid bespoke nested-field handling and keep env-var names explicit and discoverable. `figment`/TOML overlays -- config is one flat level plus a nested struct per dependency, no per-environment differences yet, so the machinery outpaces the need. Revisit on a change in *shape*, not size -- nesting past one level (e.g. `database.pool.*`), or the first value that must differ per environment -- at which point `figment` with a TOML base plus env overlay is the recorded next step.

Governs the mechanism, not the inventory: adding a value or nested group is a change to `Config` (`src/config.rs`) and `.env.example`, together the authoritative list, not an amendment here.

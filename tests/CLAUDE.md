# Tests

- Order tests by HTTP method: POST, GET, PUT, DELETE.
- Leave domain rules to the `src/entity` unit tests; cover HTTP behavior here.
- Build fixtures with direct DB/storage inserts, not by calling an endpoint the test is not about.
- Bind a chained call to a `let` before asserting on it; a lone accessor like `resp.status()` may stay inline.

## Where things live

Add a helper to the `api/helpers/` module that matches its kind. Name it after existing siblings: reuse the established verb prefixes (`get_`, `post_`, `put_`, `delete_`, `send_`) and suffixes (`_as` for a role/principal variant, `_authed` for auto-auth, `_raw` for an unwrapped request), and do not coin new terms when one already fits.

- `api/helpers/app.rs` — `TestApp` setup: construction, request dispatch, token minting.
- `api/helpers/queries.rs` — direct database and object-storage inserts and reads, corpus seeders.
- `api/helpers/http.rs` — request builders that call routes, plus response assertions.
- `api/helpers/pure.rs` — deterministic helpers that touch neither `TestApp` nor I/O.
- `api/helpers/samples.rs` — structs read back from the database for assertions.
- `api/helpers/fakers.rs` — fake-data builders and fixture constants.

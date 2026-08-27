# Tests

- Order tests by HTTP method: POST, GET, PUT, DELETE.
- Put all helper functions in `api/helpers.rs`.
- Leave domain rules to the `src/entity` unit tests; cover HTTP behavior here.
- Build fixtures with direct DB/storage inserts, not by calling an endpoint the test is not about.

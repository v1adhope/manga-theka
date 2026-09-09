# API context

HTTP contract conventions for this repo.

## Naming

- Default to plural-resource REST: `/books`, `/books/{id}`, `/books/{id}/chapters`.
- Use an RPC-style trailing verb segment when the operation is not create/read/update/delete of an addressable resource -- `POST /releases/{id}/commit`, `POST /sessions/login`. Scope the verb under its resource prefix; no flat `/auth/*` namespace.
- Short-sound rule: when both forms fit, pick the one that reads shortest and most natural aloud. REST-vs-RPC is decided per endpoint by its nature, not applied uniformly across a resource.

## Shape

- Every response uses the `{data: ...}` envelope -- single item, list, and create-ack share one shape.
- `nextCursor` is always present on list responses (no `skip_serializing_if`), `null` at end of list.
- JSON keys are `camelCase`; enum values serialize verbatim as `PascalCase`.
- Timestamps serialize as RFC 3339.
- Durations are integer seconds, not RFC 3339 -- `POST /sessions/login` and `POST /sessions/refresh` return `expiresIn` (the access token's lifetime) next to `accessToken` in the `data` envelope. The refresh token is not in the body; it rides in a `Set-Cookie` whose `Max-Age` is its own lifetime.
- Requests carry only ids for nested entities (attach-by-reference). A request-body field holding a bare id takes the `Id` suffix (`labelIds`, `creatorId`); the same concept in a response or a query-string facet takes no suffix (`labels`, `contentRating`).
- Error bodies are plain text (`"Creator not found"`), not JSON.

## Nesting

- Shallow: a collection lives under its parent (`POST`/`GET /books/{id}/chapters`), an individual member is addressed by its own id alone (`GET`/`PUT`/`DELETE /chapters/{id}`).
- Keep a `parent_id = $n` predicate on member queries while the URL asserts parentage, so a wrong-parent path 404s.

## Pagination and filtering

- Cursor-based, never offset. `?limit=`, range `[1, 100]`, default 20.
- Two cursor forms. Simple: `?after=<uuid>`, the last item's id -- the default. Complex: `?cursor=<opaque>` encoding position plus the active sort and filter selection -- for lists whose ordering is not the id order, so a bare id cannot mark the place.
- Filter facets are a fixed set of query parameters. Repeated key means OR within a facet; facets AND between each other.

## Status codes

- 400 parse failure -- malformed UUID, non-integer limit, unknown enum in a query parameter (rejected by the framework deserializer before the handler).
- 401 unauthenticated / 403 authorization fail / 404 unknown id.
- 409 conflict with the resource's current state (RFC 9110 section 15.5.10) -- duplicate unique value, illegal state transition, delete blocked by a child.
- 422 well-formed but semantically invalid content -- limit out of range, unknown enum in a JSON body.
- 413 payload too large / 415 unsupported media type, on uploads.
- 500 internal.

# Authentication and authorization: dual JWT (access + refresh) with per-session Redis session store

*Status: paper design, not implemented. ADRs 0004, 0009, and 0011 ship their features auth-free and defer the identity half of their gates to this design: `books` has no `uploaded_by`, role gates exist only as `// deferred` comments, and ADR-0009 adds a client-asserted `x-user-id`/`x-user-role` dev stand-in that must be removed in the same change that lands this middleware.*

**Model.** A `User` aggregate (distinct from `Creator`) holds a set of roles (`Reader`, `Uploader`, `Moderator`, `Admin`) as `text[]` with a CHECK constraint (`cardinality(roles) > 0`, each element in the four-role set). Reads are anonymous; writes are gated per-route by an OR role-gate -- a `from_fn_with_state` middleware injecting `AuthenticatedUser { id, sid, roles }` into request extensions.

**Tokens.** `POST /sessions` (login) issues two Ed25519-signed JWT classes:

- access: stateless, 15 min, `typ: "at+jwt"`, claims `sub`/`sid`/`role`/`iat`/`exp`, returned in the JSON body for in-memory JS storage;
- refresh: 30 day, `typ: "rt+jwt"`, claims `sub`/`sid`/`jti`/`iat`/`exp`, in an `HttpOnly; Secure; SameSite=Strict; Path=/sessions; Max-Age=2592000` cookie.

Signing keypairs are per-class, loaded from `APP_JWT_ACCESS__KEY`/`APP_JWT_REFRESH__KEY` as file paths to raw 32-byte Ed25519 seeds, public key derived at startup. `typ` at the JOSE-header level (RFC 9068 `at+jwt` plus the analogous `rt+jwt`) prevents token-class confusion; `alg` is pinned server-side to `EdDSA`, `none` rejected.

**Refresh store (Redis).** Per user: a SET `sids:<sub>` of active session ids, and per-session STRING `refresh-tokens:<sub>:<sid>` holding `{"jti": "<KeyedBLAKE3(jti)>", "ua", "ip", "createdAt", "updatedAt"}` with a native TTL of the refresh lifetime (30 d). `createdAt` is set at login and never mutated; `updatedAt` bumps on each refresh. `POST /sessions/refresh` extracts `sub`/`sid`/`jti` from the verified JWT, GETs the session JSON, compares `keyed_blake3(jti)` to the stored value; on match it issues a new refresh JWT with a fresh `jti` and pipelines `SET ... EX 2592000` + `SADD sids:<sub> <sid>` (idempotent) + `EXPIRE sids:<sub> 2592000` in one atomic `MULTI`/`EXEC`; on mismatch it `tracing::warn!`s and returns 401 -- log signal only, no reuse-detection or family-revocation escalation. `POST /sessions` uses the same pipelined write. The Redis client is `redis` with `ConnectionManager` (auto-reconnecting, multiplexed). BLAKE3 is keyed with an env pepper (`APP_BLAKE3__KEY`), distinct from the JWT signing keys.

**Session endpoints.** `DELETE /sessions/all` deletes every key from `SMEMBERS sids:<sub>` plus `DEL sids:<sub>`. `DELETE /sessions/me/current` logs the requester out via the JWT's own sid (no path sid), exempt from the 24h rule so a freshly-logged-in user can always self-logout. `DELETE /sessions/me/{sid}` revokes another of the requester's sessions, ownership-checked via `SISMEMBER sids:<sub> <sid>`. The 24h-revoke-protection rule covers `DELETE /sessions/all` and `DELETE /sessions/me/{sid}`: if the requester's session `createdAt` is under 24h old, the request is `409` -- an attacker who just compromised credentials can't immediately lock out the user's other sessions. `GET /sessions/me` reads `SMEMBERS sids:<sub>` then `MGET`s the per-session keys, returns only the still-alive pairs (`sid`/`ua`/`ip`/`createdAt`/`updatedAt`, `jti` scrubbed) and lazily `SREM`s the rest -- native TTL can clear a key before the SET catches up, so the SET is a superset filtered on read.

`me`, `current`, `all`, `refresh` are reserved path aliases and must register before the `{sid}` pattern in the router.

**Passwords.** Argon2id (`argon2` crate), params from `APP_PASSWORD__M_COST`/`__T_COST`/`__P_COST` via `envious`+`serde`, stored as the PHC string in a single `password_hash TEXT` column (per-user salt embedded, no pepper). Policy is length 16-128, no composition rules (NIST SP 800-63B). Registration (`POST /users`) is open for the default `{Reader}` role.

**Role gates.** Moderator and Admin share the content-moderation gate (`require_roles([Moderator, Admin])`); Admin alone gates user-promotion (`require_roles([Admin])`) -- the privilege gap is enforced by routing, not role hierarchy. Moderator may grant/revoke `Uploader`; Admin may grant/revoke any role, via `array_append`/`array_remove` on `users.roles` (promotion endpoint deferred from v1).

**Identifiers.** Login is by email (`POST /sessions` takes `email` though `username` is also unique). `Email` and `Username` are newtypes with `TryFrom<String>` validation in the entity layer -- `Email` via `validator::validate_email`, `Username` via `^[a-zA-Z0-9_]{3,32}$` (no `validator` derive macros, mirroring `Name`). Email verification is deferred, but `verified_at timestamptz null` is reserved on `users`.

**Routes** follow plural REST (ADR-0002) with `{data}` envelope. `/users`: `POST /users` (register, anonymous), `GET /users/me`. `/sessions`: `POST /sessions` (login, anonymous-via-credentials), `POST /sessions/refresh` (refresh-cookie; the one action sub-resource -- a pragmatic breach of pure REST), `DELETE /sessions/all`, `DELETE /sessions/me/current`, `DELETE /sessions/me/{sid}`, `GET /sessions/me`. Error map: 400 parse / 401 unauthenticated / 403 role-gate fail / 404 `sub` deleted on `/users/me` / 409 duplicate email-or-username on `POST /users` or 24h block / 422 semantic / 500 internal; 401 reveals only `"Invalid credentials"`, 403 only `"Forbidden"`. The 409-vs-422 split matches ADR-0002's RFC 9110 §15.5.10 reading. New deps: `argon2`, `jsonwebtoken`, `blake3`, `redis` (`tokio-comp`), `validator`, `regex`; `secrecy` was already present.

**Rejected.**

- `/auth/*` action namespace (RFC 6749 style) -- plural resources under `/users`/`/sessions` are consistent with ADR-0002.
- Stateful access tokens (per-token whitelist/denylist) -- preserves the "CPU-only, zero DB queries on protected requests" property, at the cost of an up-to-15-min window where a logged-out user's access JWT keeps working (mitigated by short TTL).
- Per-user HASH model -- per-session STRING keys put expiry with the data it governs, rather than split between HASH-level TTL and handler-side `expires_at`; `sids:<sub>` stays as a lookup index so `GET /sessions/me` and `DELETE /sessions/all` use `SMEMBERS`+`MGET` instead of an O(keyspace) `SCAN` (unusable around ~100k keys).
- Reuse-detection state (`previous_jti` or a used-jti SET) -- simpler session JSON, at the cost of a breach alarm the warn-log replaces.
- `citext` for email -- canonicalisation stays at the entity layer.

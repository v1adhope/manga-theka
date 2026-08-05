# Manga Theka

An API for reading and hosting user-uploaded manga, manhwa, and manhua. This glossary captures the ubiquitous language of the domain.

## Identities

**User**:
A person who operates an account. Holds a set of `Role`s, signs in with email + password, and may manage content (uploads, edits) according to those roles. Distinct from `Creator`.
_Avoid_: Account, member, profile

**Creator**:
The author or artist of a manga title -- a metadata subject, not an account operator. Referenced by books in the `creators` table; cannot log in. A `User` uploads content on behalf of a `Creator`.
_Avoid_: Author (when used to encompass artists), contributor

**Guest**:
An anonymous visitor with no account. Can read content; has no `Role` and no `Session`.
_Avoid_: Anonymous, visitor

## Authorization

**Role**:
A capability badge held by a `User`, stored as a per-user set. Four values: `Reader` (default on signup), `Uploader`, `Moderator`, `Admin`. No implicit hierarchy -- a `User` holds one or more of these at any time; granted roles can be revoked at any time.
_Avoid_: Permission, scope, claim

**Reader**:
The default `Role` on signup; gates personal features -- creating and managing bookmark lists, managing one's own account, and submitting reports for changes (e.g. corrections, metadata fixes).

**Uploader**:
A `Role` gating content-write endpoints -- a Contributor who may upload and edit content.
_Avoid_: Mod, Contributor (as a separate concept)

**Moderator**:
A content-supervisor `Role` gating content-moderation endpoints (reviewing, hiding, handling reports) and the only `Role` besides `Admin` that can change another User's role -- specifically, can grant or revoke the `Uploader` role.
_Avoid_: Mod

**Admin**:
A `Role` granting full administrative control. Gates user-promotion and administrative endpoints; can grant and revoke any `Role`.

**Session**:
A single authenticated context for a `User`. A `User` may have multiple concurrent Sessions, each revocable on its own via logout, by id, or altogether via logout-all.
_Avoid_: Login, connection

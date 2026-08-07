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

## Catalog

**Book**:
The top-level catalog entity for a manga, manhwa, or manhua work. Always has exactly one `Book Name` (English) and may have several `Alternative Title`s; `Chapter Number`s are unique within it.
_Avoid_: Title (ambiguous, see `Book Name`/`Alternative Title`), Series, Manga (too narrow -- also covers Manhwa/Manhua)

**Book Name**:
The single canonical, always-present English name of a `Book`, stored directly on the book row. The source of truth when no `Alternative Title` applies.
_Avoid_: Title, Localized name

**Alternative Title**:
A supplementary name for a `Book` in any language -- the original non-English title or an official translation. Unlike `Chapter Title`, a `Book` may have many `Alternative Title`s in the same language.
_Avoid_: Translation, Localization

**Chapter**:
An abstract, numbered slot belonging to a `Book`. Carries no language and no page images of its own -- those live on its `Chapter Release`s.
_Avoid_: Episode, Issue

**Chapter Number**:
The semantic, reader-facing identifier of a `Chapter` (e.g. "Chapter 12", "Chapter 12.5" for a special) -- editable independent of physical position, unique per `Book`.
_Avoid_: Order, Index, Sort Order

**Chapter Title**:
The canonical English-of-record name of a `Chapter`. Nullable -- not every chapter has one.
_Avoid_: Chapter Name

**Chapter Localization**:
A `Chapter Title`'s rendering in a specific language, one row per language including English (mirrored in). Unlike `Alternative Title`, at most one per language.
_Avoid_: Chapter Translation

**Label**:
A Genre or Tag applied to a `Book`, drawn from one shared catalog distinguished only by its type.
_Avoid_: Genre, Tag (as separate concepts), Category

**Book Link**:
An external URL attached to a `Book`, categorized as "Where to read", "Where to buy", or "Track".
_Avoid_: External link, Reference

## Media

**Cover**:
The `Book`'s gallery image with the lowest `Sort Order` -- not a separate flag, a derived read. Reordering the gallery can change which image is the Cover.
_Avoid_: Thumbnail, Primary image

**Sort Order**:
The physical position of a Book Cover or `Chapter Page` within its parent, starting at 1. Determines display sequence.
_Avoid_: Number, Position, Rank (see `Chapter Number` for the semantic counterpart)

**Chapter Release**:
A single-language set of `Chapter Page`s submitted in one upload. A `Chapter` may have multiple Releases -- different languages, or competing releases in the same language. The release itself (its `Chapter` and language) is fixed after creation; its pages can be individually replaced, inserted, removed, or reordered.
_Avoid_: Scan, Scanlation, Version

**Chapter Page**:
A single image belonging to a `Chapter Release`, addressed to readers by its `Sort Order` (as `page_number`) rather than its id. Individually replaceable, insertable, removable, and reorderable within its Release.
_Avoid_: Page image, Scan page

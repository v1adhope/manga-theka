# Manga Theka

An API for reading and hosting user-uploaded manga, manhwa, and manhua. This glossary captures the ubiquitous language of the domain.

## Identities

**User**:
A person who operates an account. Holds a set of `Role`s, signs in with email + password, and may manage content (uploads, edits) according to those roles. Distinct from `Creator`.
_Avoid_: Account, member, profile

**Creator**:
The author or artist of a manga title -- a metadata subject, not an account operator. Credited on any number of `Book`s, and a `Book` may credit several; cannot log in. A `User` uploads content on behalf of a `Creator`.
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
A supplementary name for a `Book` in any language -- the original non-English title or an official translation. Unlike `Chapter Localization`, a `Book` may have many `Alternative Title`s in the same language.
_Avoid_: Translation, Localization

**Book Kind**:
The publication format of a `Book` -- `Manga`, `Manhwa`, or `Manhua`. A closed, fixed set; exactly one per `Book`.
_Avoid_: Type, Format, Origin, Demographic

**Book Status**:
The publication state of the work behind a `Book` -- `Ongoing`, `Completed`, `Hiatus`, or `Cancelled`. A closed, fixed set; exactly one per `Book`. Describes the work's own progress, not the catalog record's moderation or visibility state.
_Avoid_: State, Availability, Progress

**Content Rating**:
The audience-suitability rating a `Book` carries, exactly one per Book. Unlike `Book Kind` and `Book Status`, an open set -- currently `Everyone` (E), `Teen` (T), `Teen Plus` (T+), and `Mature` (M), extensible because rating systems vary by country.
_Avoid_: Age rating, Maturity, Audience

**Publication Language**:
The language a `Book` was originally published in, exactly one per Book, drawn from the shared language catalog (ISO 639-1) that `Alternative Title` and `Chapter Release` also draw from. No `Chapter Release` may use its Book's Publication Language -- releases are always translations.
_Avoid_: Original language, Source language, Locale

**Chapter**:
An abstract, numbered slot belonging to a `Book`. Carries no language and no page images of its own -- those live on its `Chapter Release`s.
_Avoid_: Episode, Issue

**Chapter Number**:
The semantic, reader-facing identifier of a `Chapter` (e.g. "Chapter 12", "Chapter 12.5" for a special) -- editable independent of physical position, unique per `Book`.
_Avoid_: Order, Index, Sort Order

**Chapter Name**:
The canonical English-of-record name of a `Chapter`. Nullable -- not every chapter has one.
_Avoid_: Chapter Title

**Chapter Localization**:
A `Chapter Name`'s rendering in a specific language, one row per language including English (mirrored in). Unlike `Alternative Title`, at most one per language.
_Avoid_: Chapter Translation

**Chapter Volume**:
The volume a `Chapter` belongs to within its `Book`, as a whole number 0-1000. Nullable -- not every `Book` is split into volumes.
_Avoid_: Tankobon, Part, Book (in the print sense)

**Label**:
A Genre or Tag applied to a `Book`, drawn from one shared catalog distinguished only by its type.
_Avoid_: Genre, Tag (as separate concepts), Category

**Book Link**:
An external URL attached to a `Book`, categorized as "Where to read", "Where to buy", or "Track".
_Avoid_: External link, Reference

## Media

**Cover**:
The `Book`'s designated gallery image -- the one flagged as main, or, when none is flagged, the oldest image in the gallery. At most one image per `Book` carries the flag. Promoting a different image changes which one is the Cover; the rest of the gallery has no meaningful order.
_Avoid_: Thumbnail, Primary image

**Sort Order**:
The physical position of a `Chapter Page` within its `Chapter Release`, starting at 1. Determines display sequence.
_Avoid_: Number, Position, Rank (see `Chapter Number` for the semantic counterpart)

**Chapter Release**:
A single-language, ordered set of `Chapter Page`s belonging to a `Chapter`. A `Chapter` may have multiple Releases -- different languages, or competing releases in the same language. Its `Chapter` and language are fixed after creation; its pages are changed by declaring a new whole order. Carries a revision counter starting at 1, bumped once per commit (never on upload) and exposed read-only as the `version` field. Published once it holds at least one `Chapter Page`; listed for its `Chapter` from creation regardless, so one that has never been committed appears with a zero page count. Always a translation -- its language can never be the `Book`'s original publication language.
_Avoid_: Scan, Scanlation, Version, Draft (for an unpublished one)

**Chapter Page**:
A single image holding a position in its `Chapter Release`, addressed to readers by its `Sort Order` (as `page_number`) rather than its id. A Release's pages change together, as a redeclared order, rather than one at a time.
_Avoid_: Page image, Scan page

**Staged Page**:
An image uploaded into a `Chapter Release` but not yet given a `Sort Order`, and so not yet part of what readers see. Becomes a `Chapter Page` when a declared order includes it, and ceases to exist when one leaves it out.
_Avoid_: Draft page, Pending page, Unordered page

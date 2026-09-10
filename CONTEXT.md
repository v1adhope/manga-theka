# Manga Theka

A service for reading and hosting user-uploaded manga, manhwa, and manhua. This glossary captures the ubiquitous language of the domain.

## Identities

**User**:
A person who operates an account. Holds a set of `Role`s, signs in with email + password, and may manage content (uploads, edits) according to those roles. Distinct from `Creator`.
_Avoid_: Account, member, profile

**Creator**:
The author or artist of a manga title -- a metadata subject, not an account operator. Credited on any number of `Book`s, and a `Book` may credit several; cannot log in. A `User` uploads content on behalf of a `Creator`.
_Avoid_: Author (when used to encompass artists), contributor

**Guest**:
An anonymous visitor with no account. Can read content and submit `Feedback`; has no `Role` and no `Session`.
_Avoid_: Anonymous, visitor

**Created By**:
The `User` who brought an entity into being, holding standing over it that follows from having created it, not from any `Role`. A `Moderator+` holds the same standing on every entity. Applies to two entities:

- A `Book`: the `User` who proposed it as a `Draft` -- one per `Book`, the only non-`Moderator+` who can send it for review or edit it while it is a `Draft`, the audience for its `Book Note`, and able to read the `Book`, its `Chapter`s, and its `Cover`s in any `Book Visibility` -- but not the `Chapter Page`s of a `Chapter Release` they did not create.
- A `Chapter Release`: the `User` who created it -- one per `Chapter Release`; the only non-`Moderator+` who can stage `Chapter Page`s into it, commit it, delete it, read its `Chapter Page`s while its `Book` is not `Listed`, or see its staged pages.

_Avoid_: Submitter, Owner, Proposer, Uploader (a `Role`, not a relation to one entity)

## Authorization

**Role**:
A capability badge held by a `User`. Four values, ranked lowest to highest: `Reader` (default on signup), `Uploader`, `Moderator`, `Admin`. The ranking is a capability hierarchy -- a higher `Role` holds every capability of the ones below it. The notation `Role+` names that `Role` together with every higher one, so `Moderator+` means `Moderator` or `Admin`. A `User` is assigned one or more of these at any time; granted roles can be revoked at any time.
_Avoid_: Permission, scope, claim

**Reader**:
The default `Role` on signup; gates personal features -- creating and managing bookmark lists, managing one's own account, and proposing a new `Book` as a `Draft` and sending it for review (see `Book Visibility`).

**Uploader**:
A `Role` gating content writes -- editing an already-`Listed` `Book` and uploading its `Chapter`s, `Chapter Release`s, and `Chapter Page`s, none of which requires moderation. Any `Uploader` may create a `Chapter Release`, but mutating one -- staging pages, committing, deleting -- is its `Created By`'s alone (see `Created By`).
_Avoid_: Mod, Contributor (as a separate concept)

**Moderator**:
A content-supervisor `Role` gating content moderation -- reviewing, hiding, handling reports. Only a `Moderator+` can change another `User`'s roles: a `Moderator` may grant or revoke the `Uploader` role.
_Avoid_: Mod

**Admin**:
A `Role` granting full administrative control. Gates user promotion and administration; can grant and revoke any `Role`.

**Session**:
A single authenticated context for a `User`. A `User` may have multiple concurrent Sessions, each revocable on its own or all at once.
_Avoid_: Login, connection

## Catalog

**Book**:
The top-level catalog entity for a manga, manhwa, or manhua work. Always has exactly one `Book Name` (English) and may have several `Alternative Title`s; `Chapter Number`s are unique within it.
_Avoid_: Title (ambiguous, see `Book Name`/`Alternative Title`), Series, Manga (too narrow -- also covers Manhwa/Manhua)

**Book Name**:
The single canonical, always-present English name of a `Book`. The source of truth when no `Alternative Title` applies.
_Avoid_: Title, Localized name

**Alternative Title**:
A supplementary name for a `Book` in any language -- the original non-English title or an official translation. Unlike `Chapter Localization`, a `Book` may have many `Alternative Title`s in the same language.
_Avoid_: Translation, Localization

**Book Kind**:
The publication format of a `Book` -- `Manga`, `Manhwa`, or `Manhua`. A closed, fixed set; exactly one per `Book`.
_Avoid_: Type, Format, Origin, Demographic

**Book Status**:
The publication state of the work behind a `Book` -- `Ongoing`, `Completed`, `Hiatus`, or `Cancelled`. A closed, fixed set; exactly one per `Book`. Describes the work's own progress, not the catalog record's moderation or visibility state -- that is `Book Visibility`.
_Avoid_: State, Availability, Progress

**Content Rating**:
The audience-suitability rating a `Book` carries, exactly one per Book. Unlike `Book Kind` and `Book Status`, an open set -- currently `Everyone` (E), `Teen` (T), `Teen Plus` (T+), and `Mature` (M), extensible because rating systems vary by country.
_Avoid_: Age rating, Maturity, Audience

**Publication Demographic**:
The readership the original publisher marketed a `Book` to -- `Shounen`, `Shoujo`, `Seinen`, `Josei`, or `Kids`. A closed, fixed set; exactly one per `Book`. Independent of `Content Rating`.
_Avoid_: Demographic (alone), Audience, Target audience, Age rating (see `Content Rating`)

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
The canonical English-of-record name of a `Chapter`. Optional -- not every chapter has one.
_Avoid_: Chapter Title

**Chapter Localization**:
A `Chapter Name`'s rendering in a specific language, including English, which is mirrored in from the `Chapter Name`. Unlike `Alternative Title`, at most one per language.
_Avoid_: Chapter Translation

**Chapter Volume**:
The volume a `Chapter` belongs to within its `Book`, as a whole number 0-1000. Optional -- not every `Book` is split into volumes.
_Avoid_: Tankobon, Part, Book (in the print sense)

**Label**:
A curated descriptor applied to a `Book`, drawn from one shared catalog and distinguished only by its `Label Kind`. A `Genre`, a `Theme`, and a `Presentation` are all Labels, never separate entities. Read-only to every `Role` -- the catalog is seeded, never authored -- so a `Book` attaches existing Labels and never invents one.
_Avoid_: Tag, Category, Keyword

**Label Kind**:
The axis a `Label` belongs to -- `Genre`, `Theme`, or `Presentation`. A closed, fixed set; exactly one per `Label`.
_Avoid_: Type, Group, Tag

**Genre**:
A `Label Kind` naming the tradition a `Book` belongs to -- what shelf it sits on. Broad and few.
_Avoid_: Category

**Theme**:
A `Label Kind` naming a subject, setting, or trope appearing in a `Book` -- what is in it, rather than what it is. Narrow and many.
_Avoid_: Tag, Topic, Keyword

**Presentation**:
A `Label Kind` naming how a `Book` is packaged or where it came from -- its shape (`Long Strip`, `Full Color`, `Oneshot`) or its provenance (`Adaptation`, `Self-Published`). Deliberately broader than shape alone. Describes the work, never one `Chapter Release` of it.
_Avoid_: Format (see `Book Kind`), Layout, Origin

**Book Link**:
An external URL attached to a `Book`, categorized as "Where to read", "Where to buy", or "Track".
_Avoid_: External link, Reference

## Media

**Cover**:
The `Book`'s main gallery image -- the one designated as such, or, when none is designated, the oldest image in the gallery. A `Book` designates at most one. Promoting a different image changes which one is the Cover; the rest of the gallery has no meaningful order.
_Avoid_: Thumbnail, Primary image

**Sort Order**:
The physical position of a `Chapter Page` within its `Chapter Release`, starting at 1. Determines display sequence.
_Avoid_: Number, Position, Rank (see `Chapter Number` for the semantic counterpart)

**Chapter Release**:
A single-language, ordered set of `Chapter Page`s belonging to a `Chapter`. A `Chapter` may have multiple Releases -- different languages, or competing releases in the same language. Its `Chapter` and language are fixed after creation; its pages are changed by declaring a new whole order. Carries a revision counter starting at 1, bumped once each time a new order is committed and never on upload; it is never set by hand. Published once it holds at least one `Chapter Page`; listed for its `Chapter` from creation regardless, so one that has never been committed appears with a zero page count. Always a translation -- its language can never be the `Book`'s original publication language. Carries a `Created By` fixed at creation (see `Created By`).
_Avoid_: Scan, Scanlation, Version, Draft (for an unpublished one)

**Chapter Page**:
A single image holding a position in its `Chapter Release`, addressed to readers by its `Sort Order` rather than by its own identity. A Release's pages change together, as a redeclared order, rather than one at a time.
_Avoid_: Page image, Scan page

**Staged Page**:
An image uploaded into a `Chapter Release` but not yet given a `Sort Order`, and so not yet part of what readers see. Becomes a `Chapter Page` when a declared order includes it, and ceases to exist when one leaves it out.
_Avoid_: Draft page, Pending page, Unordered page

## Moderation

**Book Visibility**:
The lifecycle state of a `Book` as a catalog record, separate from `Book Status` -- `Draft` (being assembled, not yet submitted), `PendingReview` (submitted, awaiting moderation), `Listed` (approved and public), `Rejected` (declined and not retained), or `Hidden` (removed from public view by moderation). A closed, fixed set; exactly one per `Book`. Only a `Book`'s `Created By` can move it from `Draft` to `PendingReview`; a `Moderator+` owns every other transition. `PendingReview` is the only state that can return to `Draft`, so an approved or hidden `Book` never becomes a draft again. Writes follow the state: a `Draft` accepts its own metadata and gallery images -- from its `Created By` or a `Moderator+` only -- but no `Chapter`s; a `Listed` `Book` accepts everything from any `Uploader`; `PendingReview`, `Rejected`, and `Hidden` freeze the `Book` and its children alike. Reads follow the state in two tiers: a `Book`, its `Chapter`s, and its `Cover`s are readable by anyone while `Listed` or `Hidden` and otherwise only by a `Moderator+` or the `Book`'s `Created By`, with a `Chapter Release` substituting its own `Created By` for the `Book`'s; `Chapter Page` content is readable by anyone only while `Listed`, and otherwise only by a `Moderator+` or the owning `Chapter Release`'s `Created By`. A read refused for want of standing names the blocking state rather than masking it as a missing record.
_Avoid_: Status, State, Scope, Publication state, Published (see `Chapter Release`), Draft/Listed/Hidden as standalone terms

**Book Note**:
The single message carried by a `Book` and visible to its `Created By`, explaining its current `Book Visibility` -- why it was `Rejected` or `Hidden`, what to change before resubmitting, or an acknowledgement that it is awaiting review. One per `Book`, replaced whenever it changes and cleared when a `Book` becomes `Listed` without a replacement. Required on any move into `Draft`, `Rejected`, or `Hidden`; written by the system on submission. Can also be replaced on its own, leaving the `Book Visibility` unchanged. Not a private staff annotation.
_Avoid_: Review note, Comment, Reason, Moderation note, Internal note

**Feedback**:
An inbound message from a `Guest` or `User`, no `Role` required, carrying a reply-to email and a note. One of three kinds: `Report` (flags a specific `Book` for a removal-worthy problem), `Correction` (a proposed metadata fix for a specific `Book`), or `General` (site-wide feedback tied to no `Book` -- a bug report, a feature request, a question, or a message to the operators); the first two reference a `Book`, `General` does not. Progresses through `Open`, `Resolved`, and `Dismissed`, and is always retained.
_Avoid_: Complaint, Report (as the entity name), Ticket, Flag

# Manga Theka

A service for reading and hosting user-uploaded manga, manhwa, and manhua. This glossary captures the ubiquitous language of the domain. It is the authority on that language and on the rules stated here, and carries no implementation detail.

## Identities

**User**:
A person who operates an account, holding one or more `Role`s that determine what they may do. Distinct from `Creator`.
_Avoid_: Account, member, profile

**Creator**:
The author or artist of a manga title -- a metadata subject, not an account operator. Credited on any number of `Book`s under a `Creator Role`, and a `Book` may credit several; cannot sign in. A `User` uploads content on a `Creator`'s behalf.
_Avoid_: Author (when used to encompass artists), contributor

**Creator Role**:
The capacity in which a `Creator` is credited on a `Book` -- `Artist` or `Author`. A closed set; one `Creator` may hold both on the same `Book`. Unrelated to `Role`, which belongs to a `User`.
_Avoid_: Role (alone), Credit, Contribution

**Guest**:
An anonymous visitor with no account. Can read public content and submit `Feedback`; has no `Role` and no `Session`.
_Avoid_: Anonymous, visitor

## Authorization

**Role**:
A capability badge held by a `User`. Four values, ranked lowest to highest: `Reader` (default on signup), `Uploader`, `Moderator`, `Admin`. The ranking is a capability hierarchy -- a higher `Role` holds every capability of the ones below it, so the highest one a `User` holds settles what they may do. The notation `Role+` names that `Role` together with every higher one, so `Moderator+` means `Moderator` or `Admin`. A `User` holds at least one at any time; granted roles can be revoked at any time.
_Avoid_: Permission, scope, claim

**Reader**:
The `Role` gating personal features -- managing one's own account and `Bookmark List`s, and proposing a new `Book`.

**Uploader**:
The `Role` gating content writes -- creating `Chapter`s and `Chapter Release`s, and uploading `Staged Page`s. None of it requires moderation, and all of it is bounded by `Book Visibility`.
_Avoid_: Mod, Contributor (as a separate concept)

**Moderator**:
The `Role` gating content moderation -- reviewing, hiding, and handling `Feedback`. May grant or revoke the `Uploader` role.
_Avoid_: Mod

**Admin**:
The `Role` gating user administration -- promoting `User`s, and granting or revoking any `Role`.

**Created By**:
The `User` who brought an entity into being, holding standing over it that follows from having created it rather than from any `Role`. Fixed at creation, one per entity, on two entities: a `Book`, whose `Created By` proposed it as a `Draft`; and a `Chapter Release`, whose `Created By` opened it. Mutating a `Chapter Release` -- staging, committing, deleting -- and seeing its `Staged Page`s belong to its `Created By` or a `Moderator+` alone. Every other consequence of this standing follows from `Book Visibility`.
_Avoid_: Submitter, Owner, Proposer, Uploader (a `Role`, not a relation to one entity)

**Session**:
A single authenticated context for a `User`. A `User` may hold several at once, each revocable on its own or all together.
_Avoid_: Login, connection

## Catalog

**Book**:
The top-level entry for a manga, manhwa, or manhua work -- what a reader browses to, and what a `Chapter` hangs from.
_Avoid_: Title (alone -- see `Book Name`, `Alternative Title`), Series, Manga (too narrow -- also covers Manhwa/Manhua)

**Book Name**:
The single canonical, always-present English name of a `Book`. The source of truth when no `Alternative Title` applies.
_Avoid_: Title, Localized name

**Alternative Title**:
A supplementary name for a `Book` in any language -- the original non-English title or an official translation. A `Book` may carry any number, several of them in one language.
_Avoid_: Translation, Localization

**Book Kind**:
The publication format of a `Book` -- `Manga`, `Manhwa`, or `Manhua`. A closed set; exactly one per `Book`.
_Avoid_: Type, Format, Origin, Demographic

**Book Status**:
The publication state of the work behind a `Book` -- `Ongoing`, `Completed`, `Hiatus`, or `Cancelled`. A closed set; exactly one per `Book`. Describes the work's own progress, never the catalog entry's moderation state -- that is `Book Visibility`.
_Avoid_: State (alone), Availability, Progress

**Content Rating**:
The audience-suitability rating a `Book` carries -- currently `Everyone` (E), `Teen` (T), `Teen Plus` (T+), and `Mature` (M). Exactly one per `Book`; unlike `Book Kind` and `Book Status`, an open set, because rating systems vary by country.
_Avoid_: Age rating, Maturity, Audience

**Publication Demographic**:
The readership the original publisher marketed a `Book` to -- `Shounen`, `Shoujo`, `Seinen`, `Josei`, or `Kids`. A closed set; exactly one per `Book`. Independent of `Content Rating`.
_Avoid_: Demographic (alone), Audience, Target audience, Age rating (see `Content Rating`)

**Publication Language**:
The language a `Book` was originally published in, exactly one per `Book`, drawn -- like every language named in this domain -- from ISO 639-1.
_Avoid_: Original language, Source language, Locale

**Chapter**:
An abstract, numbered slot belonging to a `Book`. Carries no language and no images of its own -- those live on its `Chapter Release`s.
_Avoid_: Episode, Issue

**Chapter Number**:
The semantic, reader-facing identifier of a `Chapter` (e.g. "Chapter 12", "Chapter 12.5" for a special). Unique within its `Book`, and editable independent of any physical position.
_Avoid_: Order, Index, Sort Order (the physical counterpart, see there)

**Chapter Name**:
The canonical English-of-record name of a `Chapter`. Optional -- not every `Chapter` has one.
_Avoid_: Chapter Title

**Chapter Localization**:
A `Chapter Name`'s rendering in a specific language, including English, which is mirrored in from the `Chapter Name`. At most one per language, unlike `Alternative Title`.
_Avoid_: Chapter Translation

**Chapter Volume**:
The volume a `Chapter` belongs to within its `Book`, as a whole number. Optional -- not every work is split into volumes.
_Avoid_: Tankobon, Part, Book (in the print sense)

**Label**:
A curated descriptor applied to a `Book`, drawn from one shared, seeded vocabulary and distinguished only by its `Label Kind`. A `Genre`, a `Theme`, and a `Presentation` are all Labels, never separate entities. No `Role` authors one -- a `Book` attaches existing Labels and never invents one.
_Avoid_: Tag, Category, Keyword

**Label Kind**:
The axis a `Label` belongs to -- `Genre`, `Theme`, or `Presentation`. A closed set; exactly one per `Label`.
_Avoid_: Type, Group, Tag

**Genre**:
The `Label Kind` naming the tradition a `Book` belongs to -- what shelf it sits on. Broad and few.
_Avoid_: Category

**Theme**:
The `Label Kind` naming a subject, setting, or trope appearing in a `Book` -- what is in it, rather than what it is. Narrow and many.
_Avoid_: Tag, Topic, Keyword

**Presentation**:
The `Label Kind` naming how a `Book` is packaged or where it came from -- its shape (`Long Strip`, `Full Color`, `Oneshot`) or its provenance (`Adaptation`, `Self-Published`). Deliberately broader than shape alone. Describes the work, never one `Chapter Release` of it.
_Avoid_: Format (see `Book Kind`), Layout, Origin

**Book Link**:
An external URL attached to a `Book`, carrying exactly one `Book Link Kind`.
_Avoid_: External link, Reference

**Book Link Kind**:
What a `Book Link` points at -- `Where to read`, `Where to buy`, or `Track`. A closed set.
_Avoid_: Type, Category, Purpose

**Bookmark List**:
A named collection of `Book`s owned by one `User`, and personal to them.
_Avoid_: Library, Shelf, Favourites, Collection

## Media

**Cover**:
One of the images a `Book` carries. A `Book` may hold any number, and they have no meaningful order.
_Avoid_: Thumbnail, Gallery image, Art

**Main Cover**:
The one `Cover` a `Book` designates as its face, or, when none is designated, its oldest `Cover`. Promoting another `Cover` moves the designation.
_Avoid_: Primary image, Thumbnail, Cover (alone)

**Sort Order**:
The physical position of a `Chapter Page` within its `Chapter Release`, starting at 1. Determines display sequence.
_Avoid_: Number, Rank, Chapter Number (the semantic counterpart, see there)

**Chapter Release**:
A single-language, ordered set of `Chapter Page`s belonging to a `Chapter`. A `Chapter` may hold several -- different languages, or competing releases in the same language. Its `Chapter` and its language are fixed once it exists, and its pages change only by declaring a new whole order, which advances its revision. Always a translation: its language is never its `Book`'s `Publication Language`. Published once it holds at least one `Chapter Page`, and shown under its `Chapter` from the moment it exists, so one that has never been committed shows no pages.
_Avoid_: Scan, Scanlation, Version, Draft (for an unpublished one)

**Chapter Page**:
A single image holding a `Sort Order` in its `Chapter Release`, addressed to readers by that position rather than by its own identity.
_Avoid_: Page image, Scan page

**Staged Page**:
An image uploaded into a `Chapter Release` but not yet given a `Sort Order`, and so not yet part of what readers see. Becomes a `Chapter Page` when a declared order includes it, and ceases to exist when one leaves it out.
_Avoid_: Draft page, Pending page, Unordered page

## Moderation

**Book Visibility**:
The lifecycle state of a `Book` as a catalog entry, separate from `Book Status`. A closed set; exactly one per `Book`:

- `Draft` -- being assembled by its `Created By`, not yet submitted.
- `PendingReview` -- submitted, awaiting moderation.
- `Listed` -- approved and public.
- `Rejected` -- declined; a dead end, retained only until reclaimed.
- `Hidden` -- pulled from public reading by moderation.

Only a `Book`'s `Created By` moves it from `Draft` to `PendingReview`; a `Moderator+` owns every other transition. `PendingReview` is the only state that can return to `Draft`, so an approved or hidden `Book` never becomes a draft again, and a `Rejected` one never changes at all.

Writes follow the state: a `Draft` accepts its own metadata and `Cover`s from its `Created By` alone, not even a `Moderator+`; a `Listed` `Book` accepts everything from any `Uploader`; a `Hidden` `Book` accepts metadata and `Cover` edits from a `Moderator+` alone; `PendingReview` and `Rejected` accept nothing. `Chapter`s, `Chapter Release`s, and `Chapter Page`s are written only while their `Book` is `Listed`, and frozen in every other state with no `Moderator+` exception.

Reads follow the state in two tiers. A `Book`, its `Chapter`s, and its `Cover`s are readable by anyone while `Listed` or `Hidden`, and otherwise only by a `Moderator+` or the `Book`'s `Created By` -- a `Chapter Release` substituting its own `Created By` for the `Book`'s. `Chapter Page` content is readable by anyone only while `Listed`, and otherwise only by a `Moderator+` or the owning `Chapter Release`'s `Created By`. Hiding a `Book` is therefore a soft takedown: it withholds the pages, not the catalog entry or the art. A read refused for want of standing names the blocking state rather than masking it as a missing entry.
_Avoid_: Status (alone), State (alone), Scope, Publication state, Published (see `Chapter Release`)

**Book Note**:
The single message a `Book` carries for its `Created By`, explaining its current `Book Visibility` -- why it was `Rejected` or `Hidden`, what to change before it can be sent for review again, or an acknowledgement that it is awaiting review. One per `Book`, replaced whenever it changes and cleared when a `Book` becomes `Listed` without a replacement. Required on any move into `Draft`, `Rejected`, or `Hidden`, and written by the system on submission. May also be replaced on its own, leaving the `Book Visibility` unchanged. Not a private staff annotation.
_Avoid_: Review note, Comment, Reason, Moderation note, Internal note

**Feedback**:
An inbound message from a `Guest` or `User`, no `Role` required, carrying a reply-to email and a body. Always retained.
_Avoid_: Complaint, Report (as the entity name), Ticket, Flag, Note (for its body -- see `Book Note`)

**Feedback Kind**:
What a `Feedback` is about -- `Report` (flags a specific `Book` for a removal-worthy problem), `Correction` (a proposed metadata fix for a specific `Book`), or `General` (site-wide and tied to no `Book` -- a bug report, a feature request, a question, or a message to the operators). A closed set; exactly one per `Feedback`.
_Avoid_: Type, Category, Reason

**Feedback Status**:
How far a `Feedback` has been carried -- `Open`, `Resolved`, or `Dismissed`. A closed set; exactly one per `Feedback`.
_Avoid_: State (alone), Stage, Progress

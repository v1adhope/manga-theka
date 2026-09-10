# Book visibility is a five-state moderation lifecycle, separate from Book Status

A `Book` has a `visibility` axis -- `Draft`, `PendingReview`, `Listed`, `Rejected`, `Hidden` -- as a `CHECK`-constrained `text` column on `books` (per `docs/sql.md`), entirely separate from `books.status`. `Book Status` (`Ongoing`/`Completed`/`Hiatus`/`Cancelled`) describes the work's progress; `CONTEXT.md` scopes it to exclude moderation state, and "published" was unavailable (ADR-0006 spends it on a `Chapter Release`). Merging the two would overload a four-value set with an orthogonal meaning. This diverges from ADR-0006's "no publish flag" deliberately: a release's published state is *derivable* (has a committed page or not); "a human approved this book for the public catalog" is derivable from nothing, so it must be stored.

## Transitions

Every `Book` is created `Draft`. `POST /books` is open to any signed-in user (`Reader` and up), not just `Uploader` -- proposing a title is personal, publishing it is not. The `created_by` user's only move is `Draft -> PendingReview`, with no way to pull it back, so a moderator always reviews a state that user can't mutate underneath them. Every other transition is a moderator action through `PUT /books/{id}/visibility` with `{ visibility, note? }`, guarded by a fixed table:

- `Draft -> {PendingReview, Rejected, Hidden}`
- `PendingReview -> {Draft, Listed, Rejected, Hidden}`
- `Listed -> {Hidden, Rejected}`
- `Hidden -> {Listed, Rejected}`
- `Rejected` terminal
- every `X -> X`

`PendingReview -> Draft` is the sole way back into `Draft`: once approved or hidden a book never becomes a draft again, so `Draft` means "not yet reviewed" rather than a demotion target. An illegal cell is `409` (conflict-with-current-state, per ADR-0002); an unknown `visibility` in the body is `422`; an unknown `?visibility=` in the query is `400` (framework query deserializer, before any handler). This is the opposite of ADR-0006's per-verb RPC choice for `commit`/`upload`, deliberately: those are steps in an upload session, this is one state transition whose target the client names, so a guard table beats six near-identical handlers. The guard is a `TryFrom` in the domain layer consuming the requested transition and yielding the validated command, so an unchecked move is unrepresentable; the write is a compare-and-swap on the current visibility, not a locked read.

## Notes

A self-transition is legal and means "change only the note" -- which is why there is no `PUT /books/{id}/note`: one endpoint owns the column. Per-state `X -> X` note rules:

- `Draft`, `Rejected`, `Hidden` -- demand a non-empty note (`422` if missing) on any transition in, self-transition included.
- `Listed` -- optional note; clears the note when omitted.
- `PendingReview -> PendingReview` -- rewrites the code-authored submit message, leaves `submitted_at` alone, otherwise inert (a book under review cannot be annotated and cannot lose its queue place).

The note is written by code on `Draft -> PendingReview` (a fixed message: queued, review takes roughly X). It is a single nullable `note text` on `books`, capped at 2000 with a `check_length_books_note` backstop, reusing the existing `Text` value object (already rejects empty-or-whitespace and caps at 2000 for `Description` and `Feedback`; a `Note` newtype would duplicate those rules). It is visible to the `created_by` user, with no second staff-only field, since without auth nothing could keep one private. *Rejected: a `book_review_notes` history table (one overwritten field is enough until there's an audit log to hang history on); two booleans (`is_draft`+`is_hidden`) instead of one enum (the actor-and-reason distinction isn't recorded anywhere without an audit trail, and a single closed enum matches `kind`/`status` on this table).*

## Writes are tiered by state

- `Draft` -- accepts the `Book` row and its covers (the `created_by` user assembles metadata and artwork before review), but not chapters, releases, or pages: content cannot precede approval, so a moderator reviews a proposal rather than a library.
- `Listed` -- accepts everything.
- `PendingReview`, `Rejected`, `Hidden` -- accept nothing, so review sees a stable snapshot, a rejected book stops changing before the sweeper reaches it, and a hidden book is inert the moment a complaint lands.

Every refused write answers `409` naming the blocking state. `Hidden` being frozen reverses this ADR's first draft (which let an uploader fix the reported problem in place): a book pulled from public view for cause should not keep changing while the cause is assessed, so the repair path is `Hidden -> Listed` then an ordinary edit. `Rejected` is a dead end; a deferred sweeper reclaims rejected books and their media.

## Reads split in two

- *Metadata* is unlisted, not secret: `GET /books/{id}` and the by-id chapter and release routes serve a `Book` in any state to anyone holding the id; only `GET /books` filters, to `Listed` by default with `?visibility=<value>` overriding it for the moderator queue.
- *Page content* is not: a `Moderator` (or above) and the `Book`'s `created_by` user may read pages in any state; everyone else reads only `Listed`. `GET /releases/{id}/pages`, `/pages/{page_number}`, `/pages/{page_id}/image`, and `GET /covers/{id}/image` all answer `404` unless the owning `Book` is `Listed` or the caller can moderate -- cover art is content too.

## Auth wiring

The `Option<UserClaims>` behind every gate is populated from the access JWT by ADR-0003's global middleware (an earlier `x-user-id`/`x-user-role` dev stand-in is deleted).

Enforced (role half): `POST /books` -> any signed-in role. `PUT /books/{id}/visibility` -> a transition whose target is `PendingReview` is open to any signed-in user (the domain guard still refuses `PendingReview` from any state but `Draft`, and `PendingReview -> PendingReview` is inert); every other target requires `Moderator`/`Admin`. `GET /books?visibility=<v>` -> `Moderator`/`Admin` only; the bare list stays public, default `Listed`. The content gate's identity-free half (moderator-or-above reads any state) is `UserClaims::can_moderate`.

Deferred (`created_by` half): `chapter_pages` records no `uploaded_by`, and no handler compares `books.created_by`, so "only the `created_by` user submits their own `Draft`", "edits their own `Draft`", "reads their own non-`Listed` book and its content", and "staged pages scoped to the calling uploader" are not enforced (`// deferred` comments naming only that scope).

## Schema

`books` gains:

- `visibility text not null` -- no column `default`; the initial `Draft` is written by the insert statement (per `docs/sql.md`).
- `submitted_at timestamptz null` -- a review queue ordered by wait time needs it; the queue keeps the ordinary `id`-descending cursor and merely exposes `submittedAt`.
- `created_by uuid not null` -- FK to `users(id)` `on delete restrict`, written by `POST /books` from the caller's `UserClaims`. The `users` migration is ordered ahead of `books` so the constraint sits inline. Glossary term: **Created By**.

No `published_at`/`hidden_at` -- reconstructable from an audit log once one exists. The `Listed` scan is supported by a composite `books(visibility, id)` index, not a partial one on `where visibility = 'Listed'`: the filter is a bind parameter, so a partial index only helps when the planner can prove the parameter equals the literal, which a generic plan cannot. The composite serves the default scan and the `?visibility=` override alike, index-only, ordering matching the cursor.

`chapter_releases` and `chapter_pages` each carry their own `book_id`, denormalized from the parent, because every write and every page read must know the owning `Book`'s state: without it a release write is a two-table walk and a page read a three-table walk, with it a single join. Never client-supplied -- each insert derives it from its parent row in the same statement -- so it cannot drift. Its FK to `books` is `on delete restrict` too, so a surviving release or page blocks `DELETE /books/{id}` directly, alongside the chapter-mediated route (ADR-0006).

## Hide-request channel

The separate `Feedback` entity (its own issue): submitted by a `Guest` or `User`, no `Role`; `kind` one of `Report`/`Correction`/`General`; `status` one of `Open`/`Resolved`/`Dismissed`; optional `book_id` FK, `ON DELETE SET NULL` but non-null-required at `POST /feedbacks` for book-bound kinds. No foreign key back into the visibility machine -- a moderator reads a `Report` and calls `PUT /books/{id}/visibility`, and that transition's `note` carries the reason. `POST /feedbacks` is anonymous; other `feedbacks` routes require `[Moderator, Admin]`.

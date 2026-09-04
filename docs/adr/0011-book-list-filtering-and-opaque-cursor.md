# Book list filtering is a fixed set of query-parameter facets, paged by an opaque composite cursor

`GET /books` grows from `visibility`/`after`/`limit` into a filter over eight facets, three sorts, and two ranges, replacing the raw-UUID cursor with an opaque one. The contract:

```
labels=<uuid>                    repeated, 0..N
labelsMode=And|Or                default And
excludedLabels=<uuid>            repeated, 0..N -- no mode parameter

kind=Manga|Manhwa|Manhua                                repeated -> OR
status=Ongoing|Completed|Hiatus|Cancelled               repeated -> OR
contentRating=<uuid>                                    repeated -> OR
publicationLanguage=<uuid>                              repeated -> OR
publicationDemographic=Shounen|Shoujo|Seinen|Josei|Kids repeated -> OR
availableTranslatedLanguage=<uuid>                      repeated -> OR

publicationYearFrom=<i16>        inclusive
publicationYearTo=<i16>          inclusive
createdAtFrom=<rfc3339>          inclusive
createdAtTo=<rfc3339>            exclusive

sortField=CreatedAt|Name|PublicationYear  default CreatedAt
order=Asc|Desc                            default Desc
cursor=<opaque base64>                   replaces ?after=<uuid>
limit=1..100                             default 20

visibility=<BookVisibility>              gate still deferred to issue 3
```

**Combination.** Facets AND between them, OR within them. `labels` is the only exception, because it is the only facet where one `Book` holds many values at once -- `kind` and `status` are single-valued per book, so "all of Manga and Manhwa" is unsatisfiable and no mode parameter could mean anything. That asymmetry is why `labelsMode` exists and why the enums have no equivalent. `excludedLabels` takes no mode either, a rejection not an omission: exclusion is a blocklist and a blocklist is inherently "any of these". An `excludedLabelsMode=And` would hide only books carrying *every* excluded value at once, so a book tagged just `Gore` would still reach a reader who asked to exclude gore -- a bug with a parameter in front of it.

**Rejected richer designs.**

- A filter grammar in one parameter (`?filter=genre=in=(a,b);year=ge=2010`, RSQL/FIQL or OData style) -- more expressive and extends without new parameters, but it means owning a parser and its error messages forever, and every cache key becomes a free-text string.
- Prefix operators on values (`?labels=<uuid>&labels=-<uuid>`) -- folds exclusion into one parameter compactly, but invents a mini-syntax over values that contain the delimiter character.
- Repeated-key-means-AND with comma-means-OR -- needs no mode parameter, but makes the semantics invisible: a client library that serializes arrays differently silently changes what the query means.
- `POST /books/search` with a JSON body -- escapes URL length limits and allows nesting, at the cost of cacheability and of shareable, bookmarkable filtered views, worth more than nesting.

The chosen design is the least clever one that covers the cases, and it can grow into any of the others without breaking the ones already shipped.

**`axum::extract::Query` cannot serve this.** It deserializes with `serde_urlencoded`, which has no support for sequences, so `?labels=a&labels=b` fails to parse before any handler runs. `axum-extra`'s `Query` replaces it -- same maintainers, repeated-key semantics, no new wire syntax. This is a hard blocker, and the one new dependency this design requires.

**The ranges are deliberately asymmetric and should not be "fixed".** `publicationYearFrom`/`To` are both inclusive because a year is a discrete ordinal and `2010..2015` means what a reader thinks. `createdAtFrom` is inclusive while `createdAtTo` is exclusive, because a timestamp is continuous: a half-open interval lets adjacent ranges tile without a boundary row landing in both. All four bounds are independently optional. Only `publicationYear` and `createdAt` are filterable: `updatedAt` is nullable and would need a separate decision about whether a never-updated book falls inside a range, and "date of latest release" is not a column on `books`.

**`availableTranslatedLanguage`** answers "which books can I actually read in my language", and it is the only facet reaching outside `books` and `book_labels`. It is not `publicationLanguage`, which is the book's *original* language and says nothing about whether a translation exists; ADR-0004 forbids a release's language from equalling the publication language, so the two facets never overlap. It is expressed as a semi-join, `exists (select 1 from chapter_releases cr where cr.book_id = b.id and cr.language_id = any(...))` -- a flat probe rather than a walk through `chapters` only because ADR-0009 already denormalized `book_id` onto `chapter_releases`; an index on `chapter_releases(book_id, language_id)` makes it index-only. Its semantics are OR and only OR.

**The cursor** becomes an opaque, versioned base64 value carrying `{version, sortField, sortValue, id, filterHash}`, and `?after=<uuid>` is retired. The old cursor is a bare UUID compared against `b.id`, correct only because UUIDv7 sorts by creation time -- it silently encodes "sorted by newest" into the identifier. Every other sort key is non-unique: thousands of books share a `publicationYear`, and `where publication_year < $cursor` either skips all their ties or returns them forever. The fix is a row-wise comparison, `(b.<sortField>, b.id) < ($1, $2)`, which Postgres evaluates natively against a composite index, with `id` as the deterministic tiebreak. `filterHash` is checked on arrival and a mismatch is a `400`: paging with a cursor minted under a different filter otherwise returns quietly wrong results rather than an error. The `version` prefix lets the encoding change later without guessing what an old cursor meant. Three sorts ship -- `createdAt` (default, `Desc`), `name`, `publicationYear` -- each needing its own `books(visibility, <field>, id)` index, and that per-sort write cost is the reason the list is three and not ten.

**Doing this now rather than later is the whole point.** Retrofitting a composite cursor after launch breaks every saved link and bookmark holding a bare UUID, so it becomes a versioned migration; before launch it is free. It also settles a question that will arrive later -- sorting by rating or view count, neither of which exists in the schema today. Those need no cursor redesign (the same `(sortValue, id)` machinery covers them), but their values *change while a reader pages*, so a book already seen can drift ahead of the cursor and reappear. That drift is inherent to cursor pagination and cannot be engineered away; the mitigation is a periodically materialized rank column, stable within a session, plus one more index. Recorded here so the cursor is not blamed for it.

*Rejected: offset pagination, and page numbers with it. `OFFSET 10000` scans and discards ten thousand rows, and offsets skip or duplicate rows when writes land mid-pagination. The honest counter-argument is that this catalog is moderator-curated with rare writes and readers who seldom pass page five, which makes both drawbacks milder here -- offset lost on the deep-page cost, not unanimously. What settles it is that a cursor cannot render a page-number UI, and the catalog does not want one: "1 2 3 ... 47" needs a total row count this API deliberately does not compute. No total, no facet counts, no page numbers; `count(*)` over an arbitrary multi-label filter is frequently costlier than the page it would describe.*

**Naming.** One breaking change rides along, safe because nothing is deployed: `?after=<uuid>` becomes `?cursor=<opaque>`. `labelIds` on `POST`/`PUT /books` keeps its name -- renaming it to `labels` was tried and reverted: the `Id` suffix disambiguates not against the filter but against the *response*, where `labels` is an array of hydrated `Label` objects while the request body carries bare UUIDs. `contentRatingId`/`contentRating` and `publicationLanguageId`/`publicationLanguage` already draw that line. The rule: responses and filter facets name the concept, request-body fields carrying a bare id suffix it `Id`; a facet needs no suffix because a query string cannot carry an object. Values stay UUIDs throughout (ADR-0010 rejected a slug column), so a filter query string is UUID soup and debugging one starts with `GET /labels`.

**Loose end tightened by accident.** `Filter.sort_order` and `effective_sort_order()` already exist and are called in `database/book.rs`, but `BookListQuery` never parsed a sort parameter, so every list has silently been `Desc`. The `order` parameter finishes a path that was half-built.

**The `?visibility=` override stays ungated** (its `// deferred: gate to Moderator/Admin` comment intact), because issue 3 owns it and no `users` table exists to gate against. What this ADR adds is a constraint on that gap: no moderator-shaped facets (`note`, `submittedAt`, `hasNote`) join the filter until the gate exists -- a rich filter over an ungated visibility parameter is how `?visibility=PendingReview&limit=100` turns into an enumerable feed of unreviewed submissions, materially worse than the single-book reads ADR-0009 already accepted.

## Amendment: the label AND-match, measured

The open question this ADR left -- whether the label AND-match stays index-supported as the selection grows -- was measured against a purpose-built corpus of 1,000,000 `Book`s, 4,394,651 `book_labels` rows drawn with a realistic skew, and 488,084 `Chapter Release`s. The answer changed the implementation.

- **Grouped semi-join** (`where label_id = any(...) group by book_id having count(*) = n`) is not viable: it hides each label's selectivity behind an aggregate, so the planner must materialize the whole matching set before the outer `LIMIT`. With one popular label that is three quarters of the catalog materialized to return twenty rows: **3,176 ms**, on the likeliest query in the product.
- **Correlated `count(*) = n`** fails at the opposite end: it can drive from the sort index but cannot stop early, so a selective or empty result walks every `Listed` row at **3,286 ms**.
- **What ships: one correlated `exists` per selected label, ANDed.** Each clause is estimated independently, so the planner sees the real per-label selectivity and chooses its strategy per query -- driving from the `(visibility, <sort field>, id)` index when the filter is dense, narrowing through `book_labels(label_id, book_id)` when it is selective. Across one, two, three and five labels, common and rare, the worst case is **36 ms**. Both `book_labels` indexes are load-bearing for different plans -- the primary key serves the correlated probe, `(label_id, book_id)` serves the narrow-first one -- so neither can be retired.

**Consequence: the denormalized array column with an inverted index, which this ADR named as an escape hatch and said not to reach for first, is not needed.** Every facet, both ranges, all three sorts and a request combining all of them are under 40 ms at a million books, and the whole HTTP surface is single-digit milliseconds. Deep paging is flat -- the thousandth page of twenty costs 5.4 ms against the first page's 5.6 ms, the number the argument against offset pagination was previously missing. The consistency risk that escape hatch trades for speed buys nothing at this scale, and reopening it needs a new measurement rather than an intuition.

Caveat for whoever measures next: two of the slower cells in that run were corpus artifacts, subtle enough to have been reported as findings. `availableTranslatedLanguage` first measured 1,528 ms because every book carrying a translation happened to sit at one end of the default sort, and a `publicationYear` range under the default `createdAt` sort still measures 232 ms because that corpus derived each book's creation time from its publication year. Correlated columns in a synthetic corpus produce plans no real catalog would meet.

# Book list filtering is a fixed set of query-parameter facets, paged by an opaque composite cursor

`GET /books` is a filter over eight facets, three sorts, and two ranges, with an opaque cursor replacing the raw-UUID one. The contract:

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
cursor=<opaque base64>                    replaces ?after=<uuid>
limit=1..100                              default 20

visibility=<BookVisibility>              default Listed; =Hidden public; other values Moderator/Admin only (ADR-0009)
```

**Combination.** Facets AND between them, OR within them. `labels` is the only exception, because it is the only facet where one `Book` holds many values at once -- `kind`/`status` are single-valued per book, so "all of Manga and Manhwa" is unsatisfiable and no mode could mean anything. That asymmetry is why `labelsMode` exists and the enums have no equivalent. `excludedLabels` takes no mode either, a rejection not an omission: exclusion is a blocklist and a blocklist is inherently "any of these". An `excludedLabelsMode=And` would hide only books carrying *every* excluded value at once, so a book tagged just `Gore` would still reach a reader who asked to exclude gore.

**Rejected richer designs.**

- A filter grammar in one parameter (`?filter=genre=in=(a,b);year=ge=2010`, RSQL/FIQL or OData) -- more expressive, extends without new parameters, but means owning a parser and its error messages forever, and every cache key becomes a free-text string.
- Prefix operators on values (`?labels=<uuid>&labels=-<uuid>`) -- folds exclusion into one parameter, but invents a mini-syntax over values that contain the delimiter.
- Repeated-key-means-AND with comma-means-OR -- needs no mode parameter, but makes semantics invisible: a client library that serializes arrays differently silently changes the query's meaning.
- `POST /books/search` with a JSON body -- escapes URL length limits and allows nesting, at the cost of cacheability and shareable, bookmarkable filtered views, worth more than nesting.

The chosen design is the least clever one that covers the cases, and can grow into any of the others without breaking what shipped.

**`axum::extract::Query` cannot serve this** -- `serde_urlencoded` has no sequence support, so `?labels=a&labels=b` fails to parse before any handler runs. `axum-extra`'s `Query` replaces it (same maintainers, repeated-key semantics, no new wire syntax). The one new dependency this design requires.

**The ranges are deliberately asymmetric.** `publicationYearFrom`/`To` are both inclusive because a year is a discrete ordinal and `2010..2015` means what a reader thinks. `createdAtFrom` is inclusive, `createdAtTo` exclusive, because a timestamp is continuous: a half-open interval lets adjacent ranges tile without a boundary row landing in both. All four bounds independently optional. Only `publicationYear` and `createdAt` are filterable: `updatedAt` is nullable and would need a separate decision about a never-updated book, and "date of latest release" is not a column on `books`.

**`availableTranslatedLanguage`** answers "which books can I read in my language", the only facet reaching outside `books` and `book_labels`. It is not `publicationLanguage` (the book's *original* language, which says nothing about whether a translation exists); ADR-0004 forbids a release's language from equalling the publication language, so the two facets never overlap. Expressed as a semi-join, `exists (select 1 from chapter_releases cr where cr.book_id = b.id and cr.language_id = any(...))` -- a flat probe because ADR-0009 already denormalized `book_id` onto `chapter_releases`; an index on `chapter_releases(book_id, language_id)` makes it index-only. OR semantics only.

**The cursor** is an opaque base64 value (URL-safe, unpadded) carrying `{id, sort: {field, value}, selectionHash}`; `?after=<uuid>` is retired. The old cursor is a bare UUID compared against `b.id`, correct only because UUIDv7 sorts by creation time -- it silently encodes "sorted by newest" into the identifier. Every other sort key is non-unique: thousands of books share a `publicationYear`, and `where publication_year < $cursor` either skips all their ties or returns them forever. The fix is a row-wise comparison, `(b.<sortField>, b.id) < ($1, $2)`, evaluated natively against a composite index with `id` as the deterministic tiebreak; `sort.field` must match the requested `sortField` on arrival. `selectionHash` -- a 64-bit blake3 digest (16 hex chars) of the JSON-serialized selection (every facet plus `visibility`, `sortField`, `order`, defaults resolved before hashing) -- is checked the same way, mismatch is `400`: paging with a cursor minted under a different filter otherwise returns quietly wrong results. `limit` is outside the selection, so page size may change mid-paging. No version field: a cursor whose shape or hash no longer parses is simply a `400`, and the encoding is free to change until launch. Three sorts ship -- `createdAt` (default, `Desc`), `name`, `publicationYear` -- each needing its own `books(visibility, <field>, id)` index, and that per-sort write cost is why the list is three and not ten.

**Doing this before launch, not after.** Retrofitting a composite cursor breaks every saved link holding a bare UUID, a compatibility break to stage and migrate; before launch it is free. It also settles sorting by rating or view count later (neither in the schema today) -- the same `(sort value, id)` machinery covers them, but their values *change while a reader pages*, so a book already seen can drift ahead of the cursor and reappear. That drift is inherent to cursor pagination; the mitigation is a periodically materialized rank column, stable within a session, plus one more index.

*Rejected: offset pagination and page numbers. `OFFSET 10000` scans and discards ten thousand rows, and offsets skip or duplicate rows when writes land mid-pagination. This catalog is moderator-curated with rare writes and readers who seldom pass page five, which mutes both drawbacks -- but a cursor cannot render a page-number UI, and the catalog does not want one: "1 2 3 ... 47" needs a total row count this API deliberately does not compute. No total, no facet counts, no page numbers; `count(*)` over an arbitrary multi-label filter is frequently costlier than the page it would describe.*

**Naming.** `?after=<uuid>` becomes `?cursor=<opaque>`. `labelIds` on `POST`/`PUT /books` keeps its name -- the `Id` suffix disambiguates against the *response*, where `labels` is an array of hydrated `Label` objects while the request body carries bare UUIDs (as `contentRatingId`/`contentRating` and `publicationLanguageId`/`publicationLanguage` already do). The rule: responses and filter facets name the concept, request-body fields carrying a bare id suffix it `Id`; a facet needs no suffix because a query string cannot carry an object. Values stay UUIDs throughout (ADR-0010 rejected a slug column), so a filter query string is UUID soup and debugging one starts with `GET /labels`.

**Loose end.** `Filter.sort_order` and `effective_sort_order()` already existed and were called in `database/book.rs`, but `BookListQuery` never parsed a sort parameter, so every list had silently been `Desc`. The `order` parameter finishes that path.

## Amendment: Hidden is publicly listable (ADR-0009)

ADR-0009's read model makes a `Hidden` `Book` readable by anyone holding its id -- hiding a book is a soft takedown of its pages, not its catalog record. `GET /books?visibility=Hidden` follows: it is public, not `Moderator+`-gated. The default stays `Listed` only, and `?visibility=Draft|PendingReview|Rejected` stays `Moderator+` only, with no `created_by` path (a global list is not scoped to one caller's submissions). `selectionHash` still covers `visibility`, so a `Hidden` page and a `Listed` page never share a cursor.

## Amendment: the label AND-match, measured

Whether the label AND-match stays index-supported as the selection grows was measured against a purpose-built corpus of 1,000,000 `Book`s, 4,394,651 `book_labels` rows with realistic skew, and 488,084 `Chapter Release`s. The answer changed the implementation.

Measurement workbook: https://claude.ai/code/artifact/52af2140-7fc3-4ec4-b1f4-b4bd248a8369

- **Grouped semi-join** (`where label_id = any(...) group by book_id having count(*) = n`) -- not viable: it hides each label's selectivity behind an aggregate, so the planner materializes the whole matching set before the outer `LIMIT`. With one popular label that is three quarters of the catalog materialized to return twenty rows: **3,176 ms**, on the likeliest query in the product.
- **Correlated `count(*) = n`** -- fails at the opposite end: it drives from the sort index but can't stop early, so a selective or empty result walks every `Listed` row at **3,286 ms**.
- **What ships: one correlated `exists` per selected label, ANDed.** Each clause is estimated independently, so the planner sees real per-label selectivity and chooses its strategy per query -- driving from the `(visibility, <sort field>, id)` index when the filter is dense, narrowing through `book_labels(label_id, book_id)` when selective. Across one, two, three, five labels, common and rare, the worst case is **36 ms**. Both `book_labels` indexes are load-bearing for different plans (the primary key serves the correlated probe, `(label_id, book_id)` the narrow-first one), so neither can be retired.

**Consequence: the denormalized array column with an inverted index, named above as an escape hatch, is not needed.** Every facet, both ranges, all three sorts, and a request combining all of them are under 40 ms at a million books; the whole HTTP surface is single-digit milliseconds. Deep paging is flat -- the thousandth page of twenty costs 5.4 ms against the first page's 5.6 ms. Reopening the escape hatch needs a new measurement, not an intuition.

Caveat for whoever measures next: two slower cells in that run were corpus artifacts. `availableTranslatedLanguage` first measured 1,528 ms because every book carrying a translation happened to sit at one end of the default sort, and a `publicationYear` range under the default `createdAt` sort still measures 232 ms because that corpus derived each book's creation time from its publication year. Correlated columns in a synthetic corpus produce plans no real catalog would meet.

# Reclamation runs on pg_cron: sweep logic ships everywhere as SQL functions, the schedule is production-only

Two classes of dead data accumulate with no request to clean them up. A `Chapter Release` that is never committed again holds its `Staged Page`s indefinitely (ADR-0006 named this and deferred it). A `Rejected` `Book` is by definition not retained, but nothing deletes it. Both are unreachable by opportunistic cleanup for the same reason: the abandoned case is exactly the one nobody touches again. Reclamation therefore needs an unconditional periodic pass.

That pass is `pg_cron`, calling two plpgsql functions. Postgres runs them; the application process is not involved and is not required to be up.

**The logic ships everywhere; only the schedule is production-only.** `delete_stale_staged_chapter_pages()` and `delete_rejected_books()` are ordinary functions defined in an ordinary migration, so every environment -- dev, CI, each throwaway test database -- has them. Tests call them directly and assert on the result. `cron.schedule` calls live in `deploy/postgres/bootstrap.sql`, applied once by hand in production and nowhere else. Nothing scheduled ever runs in dev or CI, so a developer who leaves the stack up overnight never finds rows missing, and `cargo test` needs no extension. *Rejected: putting `cron.schedule` in the sqlx migrations -- they run against every throwaway test database, which would need pg_cron present and would register jobs in each.*

**Production installs pg_cron as an OS package, not an image.** Production Postgres is a standalone host install, so there is no compose file to overlay. `compose.yml` and CI share one image built `FROM postgres:18` with `postgresql-18-cron` added, and there the package sits **dormant** -- not preloaded, extension never created, no jobs. Production follows a runbook: install `postgresql-18-cron`, set `shared_preload_libraries = 'pg_cron'` and `cron.database_name = 'manga_theka'` (the default is `postgres`, and a wrong value fails silently by scheduling against the wrong database), restart, `create extension pg_cron`, then apply `bootstrap.sql`. **Every schedule change is tested by calling the function directly before it is scheduled** -- `bootstrap.sql` only registers what has already been shown to work.

This is the first component that runs application logic **outside the Rust process**. The cost is a second place to look when data goes missing; the payoff is that reclamation does not depend on the API being up, and the batching and locking behaviour stay in one transaction next to the data.

**Postgres 18 is a prerequisite, landed separately.** It provides `uuidv7()` as a built-in, matching the `Uuid::now_v7()` the application already mints everywhere. *Rejected: the `pg_uuidv7` extension (a second extension to package and enable for one function) and a hand-rolled plpgsql v7 generator (ten lines of bit-twiddling that PG18 makes redundant on arrival).*

## What each job does

**Stale staged pages, hourly.** A `Chapter Release` whose staged pages have seen no activity for two hours is taken as abandoned, and all of its staged pages are deleted. The predicate is per-release, not per-page: `group by release_id having max(created_at) < now() - interval '2 hours'`. A per-page age test would delete pages out from under an uploader still staging a long release, which ADR-0006 explicitly allows to span many requests. Committed pages are never candidates -- `sort_order is null` is the whole definition of staged.

`chapter_pages` gains `created_at timestamptz not null`, written by the application at upload like every other timestamp in the schema. *Rejected: extracting the timestamp from the UUIDv7 id -- it works, but it silently welds the sweep to the id scheme forever and reads as noise in a query file.*

**Rejected books, weekly at `0 3 * * 0` (GMT).** A `Rejected` `Book` is deleted seven days after its last change, **but only if it has no chapters**. Content-bearing ones -- reachable via `Listed -> Rejected` -- are left alone: a moderator clears their releases and chapters through the ordinary endpoints, and the next weekly run reaps the empty shell. *Rejected: making the function tear the whole subtree down itself. It would be a second implementation of teardown logic that already exists in the delete routes, and ADR-0006 refused exactly this shape -- an unbounded object purge as a side effect of an operation naming none of it. Recorded as a known gap: if `Listed -> Rejected` turns out to be common, close it by tearing content down at the transition, in the application, through the existing storage seam.*

`updated_at` is the clock. Replacing a `Book Note` on an already-`Rejected` book resets it, and that is intended: a moderator writing to a rejected book is engaging with it, and the seven days should start again.

## Objects are recorded, not deleted

Neither job talks to object storage -- `pg_cron` cannot. Both write the storage keys they strand into an append-only `orphaned_objects` table in the same statement that deletes the rows, so a swept page's image and a purged book's covers are recorded before they become unreachable. A drain worker that reads that table and deletes through the existing storage seam is **deferred**; until it lands, those objects stay in storage.

This is the first place the project deliberately deletes a row without deleting its object, and it is a smaller departure than it looks: ADR-0005 already accepts orphaned objects as harmless and client-invisible, and this trades untracked orphans for tracked ones. Without the table, reclaiming them later would mean diffing an entire bucket against the database -- the sweep ADR-0005 still defers. With it, the drain is a targeted delete by key.

`object_key` is `uuid`, not `text`: every key is a bare uuid already (a page's key is its own id, a cover's is its own id), and the narrower type measured 20% smaller on the heap. `kind` distinguishes the bucket; `source` records which job stranded it; `processed_at` is reserved for the drain and is null for now.

The load-bearing invariant is that **rows enqueued equals objects stranded**. In the 1M-row benchmark the rejected-book purge enqueued 88,247 cover keys while its cascade removed exactly 88,247 `book_covers` rows.

## Batch caps

Each function deletes at most one capped batch and returns the count; a backlog drains one batch per tick rather than in one long-locking statement. The caps come from measurement, not from a round number -- the full benchmark, with environment, dataset, query plans and the committed-drain verification, is published at <https://claude.ai/code/artifact/427c7da1-42f9-4f31-9319-4fa14aac2895> (working copy: `untracked/bench-sweeper-1m.md`, untracked).

- **Staged pages: 500 releases per run** (~10,000 pages, ~85 ms committed).
- **Rejected books: 5,000 books per run** (~345 ms committed).

The page sweep caps by **release**, not by page. Capping by page requires ordering the staged set by age, which forces a top-N sort and pushes the delete onto a full-table hash join; capping by release removes both and was measured 5.7x faster at 5,000 pages. It is also the honest unit -- a release's staged pages are one abandoned session, and sweeping part of one means nothing.

Both caps sit roughly 2x below the point where the planner abandons the nested-loop primary-key delete for a hash join against a sequential scan (1,000 releases and 20,000 books respectively), leaving room for row width and index count to grow without a run silently tipping into the expensive plan.

Each function sets `statement_timeout` locally. `pg_cron` will not start a run of a job while its previous run is still active, so a slow run delays the next rather than overlapping it. A third scheduled job prunes `cron.job_run_details` older than thirty days -- the scheduler's own log is otherwise the one table nothing reclaims.

## No new indexes

Both sweeps were measured at one million rows ([benchmark](https://claude.ai/code/artifact/427c7da1-42f9-4f31-9319-4fa14aac2895)) with and without a purpose-built index, and neither earns its place: the existing indexes already bound each sweep to its target subset. `unique_chapter_pages_release_id_sort_order` serves `sort_order is null` as an index condition, so the page sweep reads only staged rows and never the committed ones -- removing the one argument (cost scaling with the catalog) that would have justified a new index. `idx_books_visibility_created_at_id` serves `visibility = 'Rejected'` as a bitmap scan.

The candidates saved 15 ms once an hour and 10 ms once a week, against write amplification on the upload path -- a partial index on staged pages is maintained on every upload and again on every commit, as rows leave it when `sort_order` is set. Revisit only if the measured idle cost grows.

## Scope

Sessions expire on their own store's TTL (ADR-0003) and `Feedback` is retained by definition, so neither is swept. Abandoned `Draft` books are out of scope: unlike a `Rejected` one, a `Draft` is a submitter's work in progress with no statement anywhere that it should not be kept.

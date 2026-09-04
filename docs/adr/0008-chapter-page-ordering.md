# Contiguous page order is enforced by a deferrable unique constraint, because a plain unique index cannot survive the shift

`chapter_pages.sort_order` is contiguous from 1 within a release, `(release_id, sort_order)` unique. ADR-0007 dropped ordering from the covers gallery precisely because a value meaning something only relative to its siblings turns every single-row write into a statement about all of them; pages genuinely need order, so that bill comes due here. Three PostgreSQL behaviours make the obvious implementations wrong -- each established against `postgres:17` rather than reasoned about, since each contradicts a reasonable expectation.

**1. A plain unique index cannot survive a positional shift.** Reordering moves rows onto positions their siblings still hold, and PostgreSQL checks uniqueness per row *during* the statement:

```
update pa set so = so + 1 where rid = 1 and so >= 2;
ERROR:  duplicate key value violates unique constraint "u_pa"
DETAIL:  Key (rid, so)=(1, 3) already exists.
```

A row lock doesn't help -- the collision isn't between transactions, it's between rows the same statement is updating. The mirror-image shift downward happened to succeed in the same test, but only because scan order was favourable -- luck, not a guarantee, and exactly the kind of thing that passes in development and fails on a different plan.

**2. A unique index cannot be deferred; only a table constraint can.** `create unique index ... deferrable initially deferred` is a syntax error. This is why ADR-0007 couldn't defer the covers index -- a `UNIQUE` *constraint* can't be partial, so covers needed an index and had no deferral available. The page constraint is full, not partial, so it can be a table constraint (`docs/sql.md` prefers a table constraint over an index anyway):

```sql
constraint unique_chapter_pages_release_id_sort_order
	unique(release_id, sort_order) deferrable initially deferred
```

Deferral weakens nothing that matters: a genuine duplicate still raises `23505`, just at `COMMIT` rather than mid-statement.

**3. `CHECK` constraints cannot be marked `DEFERRABLE`**, eliminating the standard workaround -- a two-phase negative-offset pass (negate affected rows, then negate-and-increment them back) requiring an intermediate negative `sort_order`. With `check(sort_order between 1 and 200)` in place, the first statement is rejected outright and can't be deferred through. Keeping the plain index would have meant dropping the floor guarantee that `Sort Order` starts at 1, which `CONTEXT.md` states as part of the term's meaning.

With the deferred constraint, a commit assigns every position in one pass, no ordering trick needed: a verified case moved a page onto position 1 while another still held it, moved that one to 2 while a third held 2, and deleted three further rows -- all in one transaction that committed cleanly.

**The ceiling** is `check(sort_order between 1 and 200)`, a schema constraint not an application rule, because contiguity plus a position bound *is* a bound on the count -- no way to hold a 201st page in a contiguous set. `sort_order` is nullable, and null is the staged state: uploaded but not yet ordered by a commit. PostgreSQL treats nulls as distinct under a unique constraint, so any number of staged pages coexist, and `CHECK` passes on null, so the ceiling constrains only committed pages. A separate, looser ceiling -- `MAX_RELEASE_ROWS = 400`, application-layer, not a schema constraint -- bounds staged pages, deliberately above the committed ceiling of 200 so a full replacement can be staged while outgoing pages still hold their positions.

**Concurrency.** Mutation of one release is serialised by the row lock ADR-0006's version bump takes on `chapter_releases` as the commit transaction's first statement, not by anything in this schema. That statement lost its `and version = $2` predicate when ADR-0006 withdrew the `If-Match` precondition, but serialisation is unaffected -- the lock comes from writing the row, not the comparison. *Rejected: letting commits race and mapping `23505` to a 409 (the way ADR-0007 handles two clients promoting a cover) -- promotion is a genuine conflict where the loser learns something, while two commits to one release are a lost update where the loser's work just disappears.*

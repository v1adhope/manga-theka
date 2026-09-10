# Contiguous page order is enforced by a deferrable unique constraint, because a plain unique index cannot survive the shift

`chapter_pages.sort_order` is contiguous from 1 within a release, `(release_id, sort_order)` unique. ADR-0007 dropped ordering from the covers gallery precisely because a value meaning something only relative to its siblings turns every single-row write into a statement about all of them; pages genuinely need order, so that bill comes due here. Three PostgreSQL behaviours (each established against `postgres:17`, not reasoned about, since each contradicts a reasonable expectation) make the obvious implementations wrong:

**1. A plain unique index cannot survive a positional shift.** Reordering moves rows onto positions their siblings still hold, and PostgreSQL checks uniqueness per row *during* the statement:

```
update pa set so = so + 1 where rid = 1 and so >= 2;
ERROR:  duplicate key value violates unique constraint "u_pa"
DETAIL:  Key (rid, so)=(1, 3) already exists.
```

A row lock doesn't help -- the collision is between rows the same statement is updating, not between transactions. The mirror-image downward shift happened to succeed in the same test, but only because scan order was favourable -- luck, not a guarantee.

**2. A unique index cannot be deferred; only a table constraint can.** `create unique index ... deferrable initially deferred` is a syntax error -- this is why ADR-0007 couldn't defer the covers index. The page constraint is full, not partial, so it can be a table constraint (`docs/sql.md` prefers one anyway):

```sql
constraint unique_chapter_pages_release_id_sort_order
	unique(release_id, sort_order) deferrable initially deferred
```

Deferral weakens nothing that matters: a genuine duplicate still raises `23505`, at `COMMIT` rather than mid-statement.

**3. `CHECK` constraints cannot be `DEFERRABLE`**, eliminating the standard workaround -- a two-phase negative-offset pass needing an intermediate negative `sort_order`. With `check(sort_order between 1 and 200)` the first statement is rejected outright and can't be deferred through. Keeping the plain index would have meant dropping the floor guarantee that `Sort Order` starts at 1, which `CONTEXT.md` states as part of the term's meaning.

With the deferred constraint a commit assigns every position in one pass, no ordering trick: a verified case moved a page onto position 1 while another still held it, that one to 2 while a third held 2, and deleted three further rows -- one transaction, committed cleanly.

**The ceiling** is `check(sort_order between 1 and 200)`, a schema constraint not an application rule, because contiguity plus a position bound *is* a bound on the count. `sort_order` is nullable, and null is the staged state (uploaded but not yet ordered by a commit): PostgreSQL treats nulls as distinct under a unique constraint so any number of staged pages coexist, and `CHECK` passes on null so the ceiling constrains only committed pages. A separate, looser ceiling -- `MAX_RELEASE_ROWS = 400`, application-layer -- bounds staged pages, above the committed ceiling of 200 so a full replacement can be staged while outgoing pages still hold their positions.

**Concurrency.** Mutation of one release is serialised by the row lock ADR-0006's version bump takes on `chapter_releases` as the commit transaction's first statement, not by anything in this schema. That statement carries no `and version = $2` predicate (ADR-0006), but serialisation is unaffected -- the lock comes from writing the row. *Rejected: letting commits race and mapping `23505` to a 409 (as ADR-0007 handles two clients promoting a cover) -- promotion is a genuine conflict where the loser learns something; two commits to one release are a lost update where the loser's work just disappears.*

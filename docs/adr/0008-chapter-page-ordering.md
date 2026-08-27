# Contiguous page order is enforced by a deferrable unique constraint, because a plain unique index cannot survive the shift

`chapter_pages.sort_order` is contiguous from 1 within a release, and `(release_id, sort_order)` is unique. ADR-0007 dropped ordering from the covers gallery precisely because a value that only means something relative to its siblings turns every single-row write into a statement about all of them; pages genuinely need order, so that bill comes due here. What follows is the shape it takes, and three PostgreSQL behaviours that make the obvious implementations wrong. Each was established against `postgres:17` rather than reasoned about, because each contradicts a reasonable expectation.

**A plain unique index cannot survive a positional shift.** Reordering means moving rows onto positions their siblings still hold, and PostgreSQL checks uniqueness per row *during* the statement rather than at its end:

```
update pa set so = so + 1 where rid = 1 and so >= 2;
ERROR:  duplicate key value violates unique constraint "u_pa"
DETAIL:  Key (rid, so)=(1, 3) already exists.
```

A row lock does not help, because the collision is not between transactions -- it is between rows the same statement is updating. The mirror-image shift downward happened to succeed in the same test, but only because the scan order was favourable; that is luck, not a guarantee, and it is exactly the kind of thing that passes in development and fails on a different plan.

**A unique index cannot be deferred; only a table constraint can.** `create unique index ... deferrable initially deferred` is a syntax error. This is why ADR-0007 could not defer the covers index -- a `UNIQUE` *constraint* in PostgreSQL cannot be partial, so covers needed an index and therefore had no deferral available. The page constraint is full rather than partial, so it can be a table constraint, which is what makes deferral reachable here and unreachable there. `docs/sql.md` prefers a table constraint over an index anyway, so nothing is spent on convention:

```sql
constraint unique_chapter_pages_release_id_sort_order
	unique(release_id, sort_order) deferrable initially deferred
```

Deferral weakens nothing that matters: a genuine duplicate still raises `23505`, just at `COMMIT` rather than mid-statement.

**`CHECK` constraints cannot be marked `DEFERRABLE`**, which eliminates the standard workaround. The usual way to shift without deferral is a two-phase pass through a negative offset -- negate the affected rows, then negate and increment them back -- and that requires an intermediate state where `sort_order` is negative. With `check(sort_order between 1 and 200)` in place, the first statement is rejected outright, and the check cannot be deferred to let it through. Keeping the plain index would therefore have meant dropping the floor guarantee that `Sort Order` starts at 1, which `CONTEXT.md` states as part of what the term means.

With the deferred constraint, a commit assigns every position in one pass and needs no ordering trick: a verified case moved a page onto position 1 while another still held it, moved that one to 2 while a third held 2, and deleted three further rows, all in a single transaction that committed cleanly.

The ceiling is expressed as `check(sort_order between 1 and 200)` rather than as an application rule, because contiguity plus a bound on the position *is* a bound on the count -- there is no way to hold a 201st page in a contiguous set. `sort_order` is nullable, and that null is the staged state: a page uploaded but not yet ordered by a commit. PostgreSQL treats nulls as distinct under a unique constraint, so any number of staged pages coexist, and `CHECK` passes on null, so the ceiling constrains only committed pages. A separate, looser ceiling on total rows per release -- `MAX_RELEASE_ROWS = 400`, enforced in the application layer, not by a schema constraint -- bounds staged pages, deliberately above the committed ceiling of 200 so that a full replacement can be staged while the outgoing pages still hold their positions.

Concurrent mutation of one release is serialised by the row lock that ADR-0006's version bump takes on `chapter_releases` as the commit transaction's first statement, not by anything in this schema. That statement lost its `and version = $2` predicate when ADR-0006 withdrew the `If-Match` precondition, and the serialisation is unaffected: the lock comes from writing the row, not from the comparison. Letting commits race and mapping `23505` to a 409, the way ADR-0007 handles two clients promoting a cover, was rejected: promotion is a genuine conflict where the loser learns something, whereas two commits to one release are a lost update where the loser's work disappears.

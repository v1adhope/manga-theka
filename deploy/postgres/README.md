# Postgres deployment

Runbook for installing Reclamation's schedule. See
`docs/adr/0012-scheduled-reclamation-with-pg-cron.md` for why it is shaped this way.

## What ships where

The two sweep functions are an ordinary migration, so dev, CI and every throwaway test
database have them and the test suite calls them directly.

`pg_cron` exists in **production only**. It is not installed in dev or CI, nothing is ever
scheduled there, and the suite does not exercise it -- a stack left running overnight never
loses rows. The steps below are therefore the only place this schedule is ever tested;
step 5 is not optional.

## Installing the schedule

Production Postgres is a standalone host install, so these run by hand, once.

1. Install the extension package:

   ```sh
   apt-get install -y postgresql-18-cron
   ```

2. Add to `postgresql.conf`:

   ```
   shared_preload_libraries = 'pg_cron'
   cron.database_name = 'manga_theka'
   cron.use_background_workers = on
   ```

   `cron.database_name` defaults to `postgres`. A wrong value fails **silently**: the jobs
   register against a database that has no sweep functions, and every run errors out of
   sight. Set it explicitly.

   `cron.use_background_workers` defaults to **off**, and off is not an option here. Off
   means pg_cron runs each job over a libpq connection it opens as the job's role, which
   `pg_hba.conf` must then admit -- and the first fix anyone reaches for is a `trust` line
   for localhost, which lets any local process authenticate as any role, superuser included.
   On, pg_cron starts a background worker instead and no client authentication is involved.
   It draws from `max_worker_processes`, so leave headroom for three jobs.

3. Restart the server -- none of the three are reloadable.

4. Confirm the settings took:

   ```sql
   show shared_preload_libraries;
   show cron.database_name;
   show cron.use_background_workers;
   show cron.timezone;
   ```

   `cron.timezone` is what the schedules are read in, **not** the server's `TimeZone`. It
   defaults to `GMT`, which is what `0 3 * * 0` assumes; changing `TimeZone` does not move
   the jobs.

5. **Call each function by hand before scheduling anything.** A schedule change must never
   be the first time the logic runs in production:

   ```sql
   select delete_stale_staged_chapter_pages();
   select delete_rejected_books();
   ```

   Each returns the rows it deleted. Re-run until the count settles at 0 if there is a
   backlog -- one run deletes at most one batch (500 releases, 5000 books).

6. Apply the schedule as a superuser, connected to `manga_theka`:

   ```sh
   psql -v ON_ERROR_STOP=1 -d manga_theka -f deploy/postgres/bootstrap.sql
   ```

   `ON_ERROR_STOP=1` is load-bearing. Without it psql runs on past a failed prerequisite and
   leaves a half-applied file -- typically the role and grants in place and the two sweeps
   unscheduled, which step 7 then reports as one job instead of three.

7. Confirm all **three** jobs registered:

   ```sql
   select jobid, jobname, schedule, active, username from cron.job order by jobname;
   ```

## The jobs

| Job                          | Schedule     | What it does                                 |
| ---------------------------- | ------------ | -------------------------------------------- |
| `reclaim-stale-staged-pages` | `17 * * * *` | Sweeps staged pages of releases idle 2 hours |
| `reclaim-rejected-books`     | `0 3 * * 0`  | Purges empty books rejected 7 days ago       |
| `prune-cron-job-run-details` | `30 3 * * *` | Trims pg_cron's own run history past 30 days |

pg_cron will not start a run while the previous one is still active, so a slow run delays the
next rather than overlapping it. Each job arms `statement_timeout` in the scheduled command
rather than inside the function, because `set local` inside a function does not arm the timer
for the call already in progress.

## Who the jobs run as

`bootstrap.sql` needs a superuser (`create extension` does), but the sweeps must not *run* as
one: pg_cron records the scheduling role as the run-as role, and both function bodies are
owned by the role that runs the migrations, which is the application's. A superuser schedule
would let whoever holds the application's credentials replace a body and have Postgres run it
as superuser within the hour.

So `bootstrap.sql` schedules them as `reclamation`, a role holding nothing but `usage` on
`public`, `select`/`delete` on `chapter_pages` and `books`, `select` on `chapters` and
`book_covers`, and `insert` on `orphaned_objects`. Cascades run with the referencing table
owner's rights, so the child tables of `books` need no grant.

The role carries `LOGIN` because pg_cron refuses to schedule for a role without it, in
either execution mode. That is an attribute, not an opening: it has no password and no
`pg_hba.conf` entry, and under `cron.use_background_workers = on` nothing ever authenticates
as it. Give it neither, and do not create a host account of the same name.

This all rests on the application's own database role not being a superuser. Confirm that
before relying on any of it -- if the application connects as `postgres`, replacing a
function body is not the interesting attack.

For defence in depth, hand the functions to a role the application cannot modify
(`alter function ... owner to ...`). A later migration touching either one then has to run as
that owner.

## Watching them run

```sql
-- job outcomes
select jobid, status, return_message, start_time, end_time
from cron.job_run_details order by start_time desc limit 20;

-- what the sweeps have stranded in object storage
select kind, source, count(*) from orphaned_objects
where processed_at is null group by kind, source;
```

Each run also raises its row count to the server log, so `grep reclamation:` over the
Postgres log shows what every sweep deleted. Nothing drains `orphaned_objects` yet -- the
drain worker is deferred, and until it ships those images stay in storage as tracked orphans.

## Changing a schedule

Edit `bootstrap.sql` and re-apply it: every job is unscheduled and re-registered, so the file
is safe to run repeatedly. Grace windows and batch caps are **not** here -- they are
hard-coded in the functions, so changing one is a migration. To pause a job without removing
it, `update cron.job set active = false where jobname = '...'`; a backlog drains one batch per
tick once it is back on.

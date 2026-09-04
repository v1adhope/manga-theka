-- Reclamation schedule. Production only -- never applied in dev or CI.
--
-- Prerequisites, in order (see README.md):
--   1. postgresql-18-cron installed on the host
--   2. shared_preload_libraries = 'pg_cron', cron.database_name = 'manga_theka',
--      cron.use_background_workers = on
--   3. server restarted
--
-- Apply with `psql -v ON_ERROR_STOP=1`: a half-applied file leaves the sweeps
-- unscheduled while the prune job registers, which looks like success.
--
-- Applied by a superuser, because `create extension pg_cron` needs one. The sweeps
-- themselves must NOT run as that superuser: their function bodies are owned by the role
-- that runs the migrations, so a superuser schedule would let whoever holds the
-- application's credentials replace a body and have Postgres execute it as superuser
-- within the hour. They run as `reclamation` instead, which holds nothing but the grants
-- below.
--
-- Re-running this file is safe: every job is unscheduled first, then scheduled.
-- Never register a schedule whose function has not been called by hand first.

create extension if not exists pg_cron;

-- Without background workers pg_cron runs each job over a libpq connection it opens as the
-- job's role, which pg_hba.conf would then have to admit -- and the fix reached for first
-- is a `trust` line for localhost, which lets any local process authenticate as any role.
-- Fail here instead of leaving that trap for whoever debugs the first failed run.
do $$
begin
	if current_setting('cron.use_background_workers', true) is distinct from 'on' then
		raise exception 'cron.use_background_workers must be on'
			using hint = 'Set it in postgresql.conf and restart; see README.md step 2. '
				'Never open pg_hba.conf to the reclamation role instead.';
	end if;
end
$$;

-- LOGIN is not optional: pg_cron rejects a nologin role at schedule time, in both
-- execution modes. The role is still unusable for connecting -- it has no password and no
-- pg_hba.conf entry, and background workers bypass client authentication -- so do not
-- give it either, and do not create a host account of the same name.
do $$
begin
	if not exists (select 1 from pg_roles where rolname = 'reclamation') then
		create role reclamation login;
	end if;
end
$$;

grant usage on schema public to reclamation;
grant select, delete on chapter_pages to reclamation;
grant select, delete on books to reclamation;
grant select on chapters, book_covers to reclamation;
grant insert on orphaned_objects to reclamation;

-- Cascading deletes run with the referencing table owner's rights, so the child tables
-- of `books` need no grant of their own.

revoke execute on function delete_stale_staged_chapter_pages(), delete_rejected_books()
	from public;
grant execute on function delete_stale_staged_chapter_pages(), delete_rejected_books()
	to reclamation;

select cron.unschedule(jobid)
from cron.job
where jobname in (
	'reclaim-stale-staged-pages',
	'reclaim-rejected-books',
	'prune-cron-job-run-details'
);

select cron.schedule_in_database('reclaim-stale-staged-pages', '17 * * * *', $job$
set statement_timeout = '30s';
do $run$
declare
	v_deleted bigint;
begin
	v_deleted := delete_stale_staged_chapter_pages();
	raise log 'reclamation: swept % stale staged chapter pages', v_deleted;
end
$run$;
$job$, 'manga_theka', 'reclamation');

select cron.schedule_in_database('reclaim-rejected-books', '0 3 * * 0', $job$
set statement_timeout = '120s';
do $run$
declare
	v_deleted bigint;
begin
	v_deleted := delete_rejected_books();
	raise log 'reclamation: purged % rejected books', v_deleted;
end
$run$;
$job$, 'manga_theka', 'reclamation');

-- Prunes pg_cron's own history, which nothing else reclaims. Left on the applying role:
-- the command is a fixed statement stored in cron.job, not a function body the
-- application can replace, and cron.job_run_details belongs to the extension.
select cron.schedule('prune-cron-job-run-details', '30 3 * * *', $job$
delete from cron.job_run_details
where end_time < now() - interval '30 days'
	or (end_time is null and start_time < now() - interval '30 days');
$job$);

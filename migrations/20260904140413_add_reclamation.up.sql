create table if not exists orphaned_objects (
	id uuid,
	kind text not null,
	object_key uuid not null,
	source text not null,
	enqueued_at timestamptz not null,
	processed_at timestamptz,

	constraint pk_orphaned_objects_id primary key(id),
	constraint enum_orphaned_objects_kind check(kind in ('BookCover', 'ChapterPage')),
	constraint enum_orphaned_objects_source check(source in ('StaleStagedPage', 'RejectedBook'))
);

create index if not exists idx_orphaned_objects_enqueued_at
	on orphaned_objects(enqueued_at) where processed_at is null;

create function delete_stale_staged_chapter_pages() returns bigint as $$
declare
	v_deleted bigint;
begin
	with victim_release as (
		select p.release_id
		from chapter_pages p
		where p.sort_order is null
		group by p.release_id
		having max(p.created_at) < now() - interval '2 hours'
		limit 500
	),
	victim as (
		select p.id
		from chapter_pages p
		join victim_release vr on vr.release_id = p.release_id
		where p.sort_order is null
	),
	del as (
		delete from chapter_pages p
		using victim v
		where p.id = v.id and p.sort_order is null
		returning p.id
	),
	enqueue as (
		insert into orphaned_objects(id, kind, object_key, source, enqueued_at)
		select uuidv7(), 'ChapterPage', d.id, 'StaleStagedPage', now()
		from del d
	)
	select count(*) into v_deleted from del;

	return v_deleted;
end;
$$ language plpgsql set search_path = pg_catalog, public, pg_temp;

create function delete_rejected_books() returns bigint as $$
declare
	v_deleted bigint;
begin
	with victim as (
		select b.id
		from books b
		where b.visibility = 'Rejected'
			and b.updated_at < now() - interval '7 days'
			and not exists (select 1 from chapters c where c.book_id = b.id)
		order by b.updated_at
		limit 5000
	),
	del as (
		delete from books b
		using victim v
		where b.id = v.id
			and b.visibility = 'Rejected'
			and b.updated_at < now() - interval '7 days'
			and not exists (select 1 from chapters c where c.book_id = b.id)
		returning b.id
	),
	enqueue as (
		insert into orphaned_objects(id, kind, object_key, source, enqueued_at)
		select uuidv7(), 'BookCover', bc.id, 'RejectedBook', now()
		from book_covers bc
		join del d on d.id = bc.book_id
	)
	select count(*) into v_deleted from del;

	return v_deleted;
end;
$$ language plpgsql set search_path = pg_catalog, public, pg_temp;

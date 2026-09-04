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
	enqueue as (
		insert into orphaned_objects(id, kind, object_key, source, enqueued_at)
		select uuidv7(), 'ChapterPage', v.id, 'StaleStagedPage', now()
		from victim v
	),
	del as (
		delete from chapter_pages p
		using victim v
		where p.id = v.id
		returning p.id
	)
	select count(*) into v_deleted from del;

	return v_deleted;
end;
$$ language plpgsql;

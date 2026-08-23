create table if not exists chapter_pages (
	id uuid,
	release_id uuid not null,
	sort_order smallint,
	extension text not null,

	constraint pk_chapter_pages_id primary key(id),
	constraint fk_chapter_pages_chapter_releases_release_id foreign key(release_id) references chapter_releases(id) on delete cascade,
	constraint unique_chapter_pages_release_id_sort_order unique(release_id, sort_order) deferrable initially deferred,
	constraint check_range_chapter_pages_sort_order check(sort_order between 1 and 200),
	constraint enum_chapter_pages_extension check(extension in ('jpg', 'png', 'webp'))
);

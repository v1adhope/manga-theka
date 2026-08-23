create table if not exists chapter_releases (
	id uuid,
	chapter_id uuid not null,
	language_id uuid not null,
	version integer not null,

	constraint pk_chapter_releases_id primary key(id),
	constraint fk_chapter_releases_chapters_chapter_id foreign key(chapter_id) references chapters(id) on delete restrict,
	constraint fk_chapter_releases_languages_language_id foreign key(language_id) references languages(id) on delete restrict,
	constraint check_range_chapter_releases_version check(version >= 0)
);

create index if not exists idx_chapter_releases_chapter_id on chapter_releases(chapter_id);

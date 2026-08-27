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

create function check_chapter_releases_language_not_publication() returns trigger as $$
declare
	v_publication_language uuid;
begin
	select b.publication_language into v_publication_language
	from chapters c
	join books b on b.id = c.book_id
	where c.id = new.chapter_id;

	if v_publication_language = new.language_id then
		raise exception using
			errcode = '23514',
			constraint = 'check_chapter_releases_language_not_publication',
			message = 'chapter release language must differ from the book publication language';
	end if;

	return new;
end;
$$ language plpgsql;

create trigger check_chapter_releases_language_not_publication
	before insert or update of language_id on chapter_releases
	for each row
	execute function check_chapter_releases_language_not_publication();

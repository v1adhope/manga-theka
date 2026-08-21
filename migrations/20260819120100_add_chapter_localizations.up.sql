create table if not exists chapter_localizations (
	chapter_id uuid,
	language_id uuid,
	name text not null,

	constraint pk_chapter_localizations_chapter_id_language_id primary key(chapter_id, language_id),
	constraint fk_chapter_localizations_chapters_chapter_id foreign key(chapter_id) references chapters(id) on delete cascade,
	constraint fk_chapter_localizations_languages_language_id foreign key(language_id) references languages(id) on delete restrict,
	constraint check_length_chapter_localizations_name check(char_length(name) <= 255)
);

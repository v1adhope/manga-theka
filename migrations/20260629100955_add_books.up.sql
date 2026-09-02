create table if not exists books (
	id uuid,
	name text not null,
	description text not null,
	publication_year smallint not null,
	content_rating uuid not null,
	status text not null,
	updated_at timestamptz,
	created_at timestamptz not null,
	kind text not null,
	publication_language uuid not null,
	publication_demographic text not null,
	visibility text not null,
	note text,
	submitted_at timestamptz,

	constraint pk_books_id primary key(id),
	constraint check_length_books_name check(char_length(name) <= 255),
	constraint check_length_books_description check(char_length(description) <= 2000),
	constraint check_length_books_note check(char_length(note) <= 2000),
	constraint fk_books_content_ratings_content_rating foreign key(content_rating) references content_ratings(id) on delete restrict,
	constraint enum_books_status check(status in ('Ongoing', 'Completed', 'Hiatus', 'Cancelled')),
	constraint enum_books_kind check(kind in ('Manga', 'Manhwa', 'Manhua')),
	constraint enum_books_publication_demographic check(publication_demographic in ('Shounen', 'Shoujo', 'Seinen', 'Josei', 'Kids')),
	constraint enum_books_visibility check(visibility in ('Draft', 'PendingReview', 'Listed', 'Rejected', 'Hidden')),
	constraint fk_books_languages_publication_language foreign key(publication_language) references languages(id) on delete restrict
);

create index if not exists idx_books_visibility_created_at_id on books(visibility, created_at, id);

create index if not exists idx_books_visibility_name_id on books(visibility, name, id);

create index if not exists idx_books_visibility_publication_year_id on books(visibility, publication_year, id);

create table if not exists books (
	id uuid,
	name text not null,
	description text not null,
	publication_year smallint not null,
	content_rating uuid not null,
	status uuid not null,
	updated_at timestamptz,
	created_at timestamptz not null,
	artist uuid not null,
	author uuid not null,
	type uuid not null,
	publication_language uuid not null,

	constraint pk_books_id primary key(id),
	constraint check_length_books_name check(char_length(name) <= 255),
	constraint check_length_books_description check(char_length(description) <= 2000),
	constraint fk_books_content_ratings_content_rating foreign key(content_rating) references content_ratings(id),
	constraint fk_books_book_statuses_status foreign key(status) references book_statuses(id),
	constraint fk_books_creators_artist foreign key(artist) references creators(id),
	constraint fk_books_creators_author foreign key(author) references creators(id),
	constraint fk_books_book_types_type foreign key(type) references book_types(id),
	constraint fk_books_languages_publication_language foreign key(publication_language) references languages(id)
);

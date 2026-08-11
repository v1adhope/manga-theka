create table if not exists book_titles (
	id uuid,
	book_id uuid not null,
	language_id uuid not null,
	name text not null,

	constraint pk_book_titles_id primary key(id),
	constraint fk_book_titles_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint fk_book_titles_languages_language_id foreign key(language_id) references languages(id) on delete restrict,
	constraint check_length_book_titles_name check(char_length(name) <= 255),
	constraint unique_book_titles_book_id_language_id_name unique(book_id, language_id, name)
);

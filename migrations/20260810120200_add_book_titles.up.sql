create table if not exists book_titles (
	book_id uuid,
	language_id uuid,
	name text,

	constraint pk_book_titles_book_id_language_id_name primary key(book_id, language_id, name),
	constraint fk_book_titles_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint fk_book_titles_languages_language_id foreign key(language_id) references languages(id) on delete restrict,
	constraint check_length_book_titles_name check(char_length(name) <= 255)
);

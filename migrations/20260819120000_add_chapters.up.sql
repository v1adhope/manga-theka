create table if not exists chapters (
	id uuid,
	book_id uuid not null,
	number real not null,
	name text,
	volume smallint,
	updated_at timestamptz,
	created_at timestamptz not null,

	constraint pk_chapters_id primary key(id),
	constraint fk_chapters_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint unique_chapters_book_id_number unique(book_id, number),
	constraint check_length_chapters_name check(char_length(name) <= 255),
	constraint check_range_chapters_number check(number >= 0),
	constraint check_range_chapters_volume check(volume >= 0)
);

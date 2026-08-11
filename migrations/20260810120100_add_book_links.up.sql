create table if not exists book_links (
	id uuid,
	book_id uuid not null,
	kind text not null,
	url text not null,

	constraint pk_book_links_id primary key(id),
	constraint fk_book_links_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint enum_book_links_kind check(kind in ('WhereToRead', 'WhereToBuy', 'Track')),
	constraint check_length_book_links_url check(char_length(url) <= 2048)
);

create index if not exists idx_book_links_book_id on book_links(book_id);

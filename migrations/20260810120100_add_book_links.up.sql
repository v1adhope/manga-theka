create extension if not exists pgcrypto;

create table if not exists book_links (
	book_id uuid,
	kind text not null,
	url text not null,
	link_hash bytea generated always as (digest(url, 'sha256')) stored,

	constraint pk_book_links_book_id_link_hash primary key(book_id, link_hash),
	constraint fk_book_links_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint enum_book_links_kind check(kind in ('WhereToRead', 'WhereToBuy', 'Track')),
	constraint check_length_book_links_url check(char_length(url) <= 2048)
);

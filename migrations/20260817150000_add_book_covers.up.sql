create table if not exists book_covers (
	id uuid,
	book_id uuid not null,
	extension text not null,
	is_main boolean not null,

	constraint pk_book_covers_id primary key(id),
	constraint fk_book_covers_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint enum_book_covers_extension check(extension in ('jpg', 'png', 'webp'))
);

create index if not exists idx_book_covers_book_id on book_covers(book_id);

create unique index if not exists unique_book_covers_book_id_is_main
	on book_covers(book_id) where is_main;

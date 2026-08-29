create table if not exists book_creators (
	book_id uuid,
	creator_id uuid,
	role text not null,

	constraint pk_book_creators_book_id_creator_id_role primary key(book_id, creator_id, role),
	constraint fk_book_creators_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint fk_book_creators_creators_creator_id foreign key(creator_id) references creators(id) on delete restrict,
	constraint enum_book_creators_role check(role in ('Artist', 'Author'))
);

create index idx_book_creators_creator_id on book_creators(creator_id) include (role);

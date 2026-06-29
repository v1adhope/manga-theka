create table if not exists book_types(
	id uuid,
	name text not null,

	constraint pk_book_types_id primary key(id),
	constraint check_length_book_types_name check(char_length(name) <= 255),
	constraint unique_book_types_name unique(name)
);

insert
	into
	book_types (id,
	name)
values ('019f129b-652e-7498-831b-9d95d54626f3',
'Manga'),
('019f129b-766c-7389-b413-dfc1ad9755f1',
'Manhwa'),
('019f129b-8ce9-7cb7-908d-4749552efb16',
'Manhua') on
conflict(name) do nothing;

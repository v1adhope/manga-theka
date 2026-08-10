create table if not exists book_statuses (
	id uuid,
	name text not null,

	constraint pk_book_statuses_id primary key(id),
	constraint check_length_book_statuses_name check(char_length(name) <= 255),
	constraint unique_book_statuses_name unique(name)
);

insert
	into
	book_statuses (id,
	name)
values ('019f1274-8324-7edf-a633-5150f84e1840',
'Ongoing'),
('019f1275-0d9c-7466-8ab6-9eb15fd0fc1f',
'Completed'),
('019f1275-3db0-73ed-acbe-ae823e4b4e8d',
'Hiatus'),
('019f1275-6f97-74de-8793-c4c146125b28',
'Cancelled') on
conflict(name) do nothing;

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

create table if not exists label_types (
	id uuid,
	name text not null,

	constraint pk_label_types_id primary key(id),
	constraint check_length_label_types_name check(char_length(name) <= 255),
	constraint unique_label_types_name unique(name)
);

insert
	into
	label_types (id,
	name)
values ('019f12c9-2d2d-7027-ad30-d1c22b97acf0',
'Genre'),
('019f12c9-3d9c-7937-8e50-bc0116bc53b4',
'Tag') on
conflict(name) do nothing;

create table if not exists images (
	id uuid,
	entity_id uuid not null,
	entity_type text not null,
	extention text not null,
	sort_order smallint not null,

	constraint pk_images_id primary key(id),
	constraint check_length_images_entity_type check(char_length(entity_type) <= 255),
	constraint check_length_images_extention check(char_length(extention) <= 255)
);

alter table books add column status_new uuid;
update books set status_new = book_statuses.id from book_statuses where book_statuses.name = books.status;
alter table books alter column status_new set not null;
alter table books drop constraint enum_books_status;
alter table books drop column status;
alter table books rename column status_new to status;
alter table books add constraint fk_books_book_statuses_status foreign key(status) references book_statuses(id);

alter table books add column type_new uuid;
update books set type_new = book_types.id from book_types where book_types.name = books.type;
alter table books alter column type_new set not null;
alter table books drop constraint enum_books_type;
alter table books drop column type;
alter table books rename column type_new to type;
alter table books add constraint fk_books_book_types_type foreign key(type) references book_types(id);

alter table labels add column type_new uuid;
update labels set type_new = label_types.id from label_types where label_types.name = labels.type;
alter table labels alter column type_new set not null;
alter table labels drop constraint enum_labels_type;
alter table labels drop column type;
alter table labels rename column type_new to type;
alter table labels add constraint fk_labels_label_types_type foreign key(type) references label_types(id);

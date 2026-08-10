alter table books add column status_new text;
update books set status_new = book_statuses.name from book_statuses where book_statuses.id = books.status;
alter table books alter column status_new set not null;
alter table books drop constraint fk_books_book_statuses_status;
alter table books drop column status;
alter table books rename column status_new to status;
alter table books add constraint enum_books_status check (status in ('Ongoing', 'Completed', 'Hiatus', 'Cancelled'));

alter table books add column type_new text;
update books set type_new = book_types.name from book_types where book_types.id = books.type;
alter table books alter column type_new set not null;
alter table books drop constraint fk_books_book_types_type;
alter table books drop column type;
alter table books rename column type_new to type;
alter table books add constraint enum_books_type check (type in ('Manga', 'Manhwa', 'Manhua'));

alter table labels add column type_new text;
update labels set type_new = label_types.name from label_types where label_types.id = labels.type;
alter table labels alter column type_new set not null;
alter table labels drop constraint fk_labels_label_types_type;
alter table labels drop column type;
alter table labels rename column type_new to type;
alter table labels add constraint enum_labels_type check (type in ('Genre', 'Tag'));

drop table if exists book_statuses;
drop table if exists book_types;
drop table if exists label_types;
drop table if exists images;

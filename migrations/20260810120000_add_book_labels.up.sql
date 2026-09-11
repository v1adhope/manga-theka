create table if not exists book_labels (
	book_id uuid,
	label_id uuid,

	constraint pk_book_labels_book_id_label_id primary key(book_id, label_id),
	constraint fk_book_labels_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint fk_book_labels_labels_label_id foreign key(label_id) references labels(id) on delete restrict
);

create index if not exists idx_book_labels_label_id_book_id on book_labels(label_id, book_id);

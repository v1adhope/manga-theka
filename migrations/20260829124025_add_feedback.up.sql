create table if not exists feedback (
	id uuid,
	kind text not null,
	status text not null,
	email text not null,
	note text not null,
	book_id uuid,
	updated_at timestamptz,
	created_at timestamptz not null,

	constraint pk_feedback_id primary key(id),
	constraint enum_feedback_kind check(kind in ('Report', 'Correction', 'General')),
	constraint enum_feedback_status check(status in ('Open', 'Resolved', 'Dismissed')),
	constraint check_length_feedback_email check(char_length(email) <= 254),
	constraint check_length_feedback_note check(char_length(note) <= 2000),
	constraint fk_feedback_books_book_id foreign key(book_id) references books(id) on delete set null
);

create index if not exists idx_feedback_book_id on feedback(book_id);

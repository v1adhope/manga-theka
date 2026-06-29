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

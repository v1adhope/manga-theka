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

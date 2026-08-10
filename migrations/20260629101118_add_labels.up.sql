create table if not exists labels (
	id uuid,
	name text not null,
	type text not null,

	constraint pk_labels_id primary key(id),
	constraint check_length_labels_name check(char_length(name) <= 255),
	constraint enum_labels_type check(type in ('Genre', 'Tag'))
);

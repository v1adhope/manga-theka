create table if not exists labels (
	id uuid,
	name text not null,
	kind text not null,

	constraint pk_labels_id primary key(id),
	constraint check_length_labels_name check(char_length(name) <= 255),
	constraint enum_labels_kind check(kind in ('Genre', 'Tag'))
);

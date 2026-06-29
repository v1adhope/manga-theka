create table if not exists labels (
	id uuid,
	name text not null,
	type uuid not null,

	constraint pk_labels_id primary key(id),
	constraint check_length_labels_name check(char_length(name) <= 255),
	constraint fk_labels_label_types_type foreign key(type) references label_types(id)
);

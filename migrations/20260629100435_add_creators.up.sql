create table if not exists creators (
	id uuid,
	first_name text not null,
	last_name text not null,
	role text not null,
	created_at timestamptz not null,

	constraint pk_creators_id primary key(id),
	constraint check_length_creators_first_name check(char_length(first_name) <= 255),
	constraint check_length_creators_last_name check(char_length(last_name) <= 255),
  constraint enum_creators_role check(role in ('Artist', 'Author')),
  constraint unique_creators_first_name_last_name unique (first_name, last_name)
);

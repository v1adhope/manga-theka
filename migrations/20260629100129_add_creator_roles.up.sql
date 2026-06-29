create table if not exists creator_roles (
	id uuid,
	name text not null,

	constraint pk_creator_roles_id primary key(id),
	constraint check_length_creator_roles_name check(char_length(name) <= 255),
	constraint unique_creator_roles_name unique(name)
);

insert
	into
	creator_roles (id,
	name)
values ('019f1229-fabf-7770-a19b-b4f806c5044f',
'Artist'),
('019f122a-5ff5-76b6-b509-500cc328972c',
'Author') on
conflict(name) do nothing;

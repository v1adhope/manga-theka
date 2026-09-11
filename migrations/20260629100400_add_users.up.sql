create table if not exists users (
	id uuid,
	email text not null,
	username text not null,
	password_hash text not null,
	roles text[] not null,
	verified_at timestamptz,
	created_at timestamptz not null,

	constraint pk_users_id primary key(id),
	constraint unique_users_email unique(email),
	constraint unique_users_username unique(username),
	constraint check_length_users_email check(char_length(email) <= 254),
	constraint check_length_users_username check(char_length(username) <= 32),
	constraint check_length_users_password_hash check(char_length(password_hash) <= 255),
	constraint check_empty_users_roles check(cardinality(roles) > 0),
	constraint enum_users_roles check(roles <@ array['Reader', 'Uploader', 'Moderator', 'Admin']::text[])
);
